//! Calendar recurrence and reminder decisions, independent of windows or timers.
use chrono::{DateTime, Days, Local, Months, NaiveDateTime, TimeDelta, TimeZone};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::Item;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Frequency {
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repeat {
    pub frequency: Frequency,
    /// Original anchors never move: January 31 -> February 28 -> March 31.
    pub reset_anchor: i64,
    pub due_anchor: Option<i64>,
    #[serde(default)]
    pub occurrence: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub due_at: Option<i64>,
    pub remind_before_minutes: Option<u32>,
    pub repeat: Option<Repeat>,
    /// Persisted across restarts; completing/uncompleting does not re-alert.
    pub notified_for: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInput {
    pub due_at: Option<i64>,
    pub remind_before_minutes: Option<u32>,
    pub frequency: Option<Frequency>,
    pub next_reset_at: Option<i64>,
}

fn valid_time(value: i64) -> bool {
    // 2000 through 2100. Bounds also keep catch-up work and arithmetic bounded.
    (946684800000..4133980800000).contains(&value)
}

pub fn configure(old: &Schedule, input: ScheduleInput, now: i64) -> Result<Schedule> {
    if input.due_at.is_some_and(|v| !valid_time(v))
        || input.next_reset_at.is_some_and(|v| !valid_time(v))
    {
        return Err(Error::Invalid("日期须在 2000～2100 年之间".into()));
    }
    if input.remind_before_minutes.is_some_and(|v| v > 525600)
        || (input.remind_before_minutes.is_some() && input.due_at.is_none())
    {
        return Err(Error::Invalid(
            "提醒需要截止时间，提前量不能超过一年".into(),
        ));
    }
    let repeat = match (input.frequency, input.next_reset_at) {
        (Some(frequency), Some(reset_anchor)) => {
            if reset_anchor <= now || input.due_at.is_some_and(|due| due >= reset_anchor) {
                return Err(Error::Invalid(
                    "下次重置须晚于当前时间及本期截止时间".into(),
                ));
            }
            // Preserve the original day-of-month when reopening and saving.
            if let Some(r) = &old.repeat {
                if r.frequency == frequency
                    && next_reset(r) == Some(reset_anchor)
                    && input.due_at == old.due_at
                {
                    Some(r.clone())
                } else {
                    Some(Repeat {
                        frequency,
                        reset_anchor,
                        due_anchor: input.due_at,
                        occurrence: 0,
                    })
                }
            } else {
                Some(Repeat {
                    frequency,
                    reset_anchor,
                    due_anchor: input.due_at,
                    occurrence: 0,
                })
            }
        }
        (None, None) => None,
        _ => return Err(Error::Invalid("循环任务需要设置下次重置时间".into())),
    };
    Ok(Schedule {
        due_at: input.due_at,
        remind_before_minutes: input.remind_before_minutes,
        notified_for: if old.due_at == input.due_at
            && old.remind_before_minutes == input.remind_before_minutes
        {
            old.notified_for
        } else {
            None
        },
        repeat,
    })
}

fn calendar_shift(anchor: NaiveDateTime, frequency: Frequency, n: u32) -> Option<NaiveDateTime> {
    match frequency {
        Frequency::Weekly => anchor.checked_add_days(Days::new(u64::from(n) * 7)),
        Frequency::Monthly => anchor.checked_add_months(Months::new(n)),
    }
}

pub fn shift(anchor: i64, frequency: Frequency, n: u32) -> Option<i64> {
    let local = DateTime::from_timestamp_millis(anchor)?.with_timezone(&Local);
    let target = calendar_shift(local.naive_local(), frequency, n)?;
    // Choose the first instant on a repeated hour. On a DST gap, move forward
    // to the first valid minute instead of silently dropping this occurrence.
    for minute in 0..=180 {
        let candidate = target.checked_add_signed(TimeDelta::minutes(minute))?;
        if let Some(value) = Local.from_local_datetime(&candidate).earliest() {
            return Some(value.timestamp_millis());
        }
    }
    None
}

pub fn next_reset(repeat: &Repeat) -> Option<i64> {
    shift(repeat.reset_anchor, repeat.frequency, repeat.occurrence)
}

pub fn advance(item: &mut Item, now: i64) -> bool {
    let Some(repeat) = &mut item.schedule.repeat else {
        return false;
    };
    let before = repeat.occurrence;
    while repeat.occurrence < 12000 && next_reset(repeat).is_some_and(|at| at <= now) {
        repeat.occurrence += 1;
    }
    if before == repeat.occurrence {
        return false;
    }
    item.schedule.due_at = repeat
        .due_anchor
        .and_then(|at| shift(at, repeat.frequency, repeat.occurrence));
    item.schedule.notified_for = None;
    item.done = false;
    true
}

pub fn urgent(item: &Item, now: i64) -> bool {
    !item.done
        && item.schedule.due_at.is_some_and(|due| {
            now >= due - i64::from(item.schedule.remind_before_minutes.unwrap_or(0)) * 60000
        })
}

pub fn needs_reminder(item: &Item, now: i64) -> bool {
    urgent(item, now)
        && item.schedule.remind_before_minutes.is_some()
        && item.schedule.notified_for != item.schedule.due_at
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> i64 {
        Local
            .from_local_datetime(&text.parse::<NaiveDateTime>().unwrap())
            .earliest()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn old_items_load_without_schedule() {
        let item: Item =
            serde_json::from_str(r#"{"id":"a","text":"old","done":true,"createdAt":1}"#).unwrap();
        assert_eq!(item.schedule, Schedule::default());
        assert!(item.done);
    }

    #[test]
    fn months_clamp_without_drifting_and_handle_leap_years() {
        let jan: NaiveDateTime = "2024-01-31T10:00:00".parse().unwrap();
        assert_eq!(
            calendar_shift(jan, Frequency::Monthly, 1)
                .unwrap()
                .to_string(),
            "2024-02-29 10:00:00"
        );
        assert_eq!(
            calendar_shift(jan, Frequency::Monthly, 2)
                .unwrap()
                .to_string(),
            "2024-03-31 10:00:00"
        );
        assert_eq!(
            calendar_shift(jan, Frequency::Monthly, 13)
                .unwrap()
                .to_string(),
            "2025-02-28 10:00:00"
        );
    }

    #[test]
    fn catch_up_only_resets_once_to_current_cycle_and_moves_deadline() {
        let mut item = Item::new("weekly".into());
        item.done = true;
        let due = at("2026-11-06T18:00:00");
        item.schedule = Schedule {
            due_at: Some(due),
            remind_before_minutes: Some(60),
            notified_for: Some(due),
            repeat: Some(Repeat {
                frequency: Frequency::Weekly,
                reset_anchor: at("2026-11-09T09:00:00"),
                due_anchor: Some(due),
                occurrence: 0,
            }),
        };
        assert!(!advance(&mut item, at("2026-11-08T09:00:00")));
        assert!(item.done);
        assert!(advance(&mut item, at("2026-11-23T09:00:00")));
        assert!(!item.done);
        assert_eq!(item.schedule.due_at, Some(at("2026-11-27T18:00:00")));
        assert_eq!(
            next_reset(item.schedule.repeat.as_ref().unwrap()),
            Some(at("2026-11-30T09:00:00"))
        );
        assert!(!advance(&mut item, at("2026-11-23T09:00:00")));
        assert_eq!(item.schedule.notified_for, None);
    }

    #[test]
    fn reminders_are_due_once_and_completed_items_are_never_urgent() {
        let mut item = Item::new("deadline".into());
        item.schedule.due_at = Some(120000);
        item.schedule.remind_before_minutes = Some(1);
        assert!(!needs_reminder(&item, 59999));
        assert!(needs_reminder(&item, 60000));
        item.schedule.notified_for = Some(120000);
        let saved = serde_json::to_string(&item).unwrap();
        item = serde_json::from_str(&saved).unwrap();
        assert!(!needs_reminder(&item, 130000));
        assert!(urgent(&item, 130000));
        item.done = true;
        assert!(!urgent(&item, 130000));
        item.done = false;
        assert!(!needs_reminder(&item, 130000));
        item.schedule.remind_before_minutes = None;
        assert!(!urgent(&item, 119999));
        assert!(urgent(&item, 120000));
    }

    #[test]
    fn rejects_invalid_schedule_and_preserves_anchor_when_saving_unchanged() {
        let now = at("2026-11-01T00:00:00");
        let input = ScheduleInput {
            due_at: None,
            remind_before_minutes: Some(10),
            frequency: None,
            next_reset_at: None,
        };
        assert!(configure(&Schedule::default(), input, now).is_err());
        let old = Schedule {
            due_at: Some(at("2026-11-28T10:00:00")),
            repeat: Some(Repeat {
                frequency: Frequency::Monthly,
                reset_anchor: at("2026-01-31T10:00:00"),
                due_anchor: Some(at("2026-01-28T10:00:00")),
                occurrence: 10,
            }),
            ..Schedule::default()
        };
        let input = ScheduleInput {
            due_at: old.due_at,
            remind_before_minutes: None,
            frequency: Some(Frequency::Monthly),
            next_reset_at: next_reset(old.repeat.as_ref().unwrap()),
        };
        assert_eq!(configure(&old, input, now).unwrap(), old);
    }
}
