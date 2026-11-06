use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;

use crate::error::Result;
use crate::model::{now_ms, Workspace, SCHEMA_VERSION};
use crate::schedule;

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Badge {
    pub count: usize,
    pub urgent: usize,
}

pub fn badge(ws: &Workspace, now: i64) -> Badge {
    Badge {
        count: ws.open_count(),
        urgent: ws
            .cards
            .iter()
            .flat_map(|c| &c.items)
            .filter(|i| schedule::urgent(i, now))
            .count(),
    }
}

/// Single-document, crash-safe persistence for the workspace.
///
/// Writes go to a sibling temp file that is flushed and then renamed over the
/// real one, so a power loss mid-save leaves the previous good document intact
/// rather than a half-written one. `Store` is the only place that touches disk;
/// swapping in a different backend means reimplementing `load`/`persist` alone.
/// Called after every successful mutation with the number of unfinished items.
///
/// A hook on the store rather than a call at the end of each command: there are
/// eight mutating commands plus the tray, and "remember to notify" is exactly the
/// kind of step that gets forgotten when a ninth is added.
pub type ChangeHook = Box<dyn Fn(Badge) + Send + Sync>;

pub struct Store {
    path: PathBuf,
    inner: RwLock<Workspace>,
    on_change: ChangeHook,
}

impl Store {
    /// Loads the workspace at `path`, falling back to a fresh one. A file that
    /// exists but cannot be parsed is preserved under a `.corrupt-<ts>` name so
    /// the user's notes are never silently discarded.
    pub fn load(path: PathBuf, on_change: ChangeHook) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut workspace = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Workspace>(&raw) {
                Ok(ws) => ws,
                Err(err) => {
                    let quarantine = path.with_extension(format!("corrupt-{}", now_ms()));
                    log::error!(
                        "workspace at {} is unreadable ({err}); moved to {}",
                        path.display(),
                        quarantine.display()
                    );
                    let _ = fs::rename(&path, &quarantine);
                    Workspace::default()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Workspace::default(),
            Err(err) => return Err(err.into()),
        };

        workspace.version = SCHEMA_VERSION;
        workspace.normalise();

        let store = Self {
            path,
            inner: RwLock::new(workspace),
            on_change,
        };
        store.flush()?;
        Ok(store)
    }

    /// Reads a projection of the workspace without cloning the whole document.
    pub fn read<R>(&self, f: impl FnOnce(&Workspace) -> R) -> R {
        f(&self.inner.read())
    }

    pub fn snapshot(&self) -> Workspace {
        self.inner.read().clone()
    }

    /// Mutates the workspace and persists the result before returning. The
    /// invariant check runs on every mutation so no command can leave the UI
    /// pointing at a card that no longer exists.
    pub fn write<R>(&self, f: impl FnOnce(&mut Workspace) -> Result<R>) -> Result<R> {
        let (out, open) = {
            let mut guard = self.inner.write();
            // Commit the in-memory state only after the disk write succeeds.
            let mut next = guard.clone();
            let out = f(&mut next)?;
            next.normalise();
            persist(&self.path, &next)?;
            *guard = next;
            (out, badge(&guard, now_ms()))
        };
        // Deliberately outside the lock: the hook reaches into Tauri, and a
        // listener that reads the store back would deadlock against our own
        // write guard.
        (self.on_change)(open);
        Ok(out)
    }

    /// Called by one native worker, even while both webviews are hidden.
    /// No file writes when nothing changed. Holding the lock prevents a
    /// concurrent completion/deletion from racing the decision to send.
    pub fn process_time(
        &self,
        now: i64,
        mut send: impl FnMut(&str, &str) -> Result<()>,
    ) -> Result<bool> {
        let (changed, state) = {
            let mut guard = self.inner.write();
            let mut next = guard.clone();
            let mut changed = false;
            for card in &mut next.cards {
                for item in &mut card.items {
                    if schedule::advance(item, now) {
                        changed = true;
                        card.updated_at = now;
                    }
                    if schedule::needs_reminder(item, now) {
                        match send(&card.title, &item.text) {
                            Ok(()) => {
                                item.schedule.notified_for = item.schedule.due_at;
                                changed = true;
                            }
                            Err(err) => log::warn!("reminder delivery failed: {err}"),
                        }
                    }
                }
            }
            if changed {
                persist(&self.path, &next)?;
                *guard = next;
            }
            (changed, badge(&guard, now))
        };
        (self.on_change)(state);
        Ok(changed)
    }

    fn flush(&self) -> Result<()> {
        persist(&self.path, &self.inner.read())
    }
}

fn persist(path: &Path, workspace: &Workspace) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(workspace)?;

    {
        let mut file = File::create(&tmp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }

    // Atomic replace on both platforms: POSIX rename(2) semantics, and
    // MoveFileExW with MOVEFILE_REPLACE_EXISTING on Windows.
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::model::Item;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("jotter-test-{}", uuid::Uuid::new_v4())))
        }
        fn load(&self) -> Store {
            Store::load(self.0.join("workspace.json"), Box::new(|_| {})).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reminder_success_persists_and_failure_can_retry() {
        let fixture = Fixture::new();
        let store = fixture.load();
        store
            .write(|ws| {
                let mut item = Item::new("remind".into());
                item.schedule.due_at = Some(100000);
                item.schedule.remind_before_minutes = Some(1);
                ws.cards[0].items.push(item);
                Ok(())
            })
            .unwrap();
        assert!(!store
            .process_time(40000, |_, _| Err(Error::Invalid("disabled".into())))
            .unwrap());
        assert!(store.read(|ws| schedule::needs_reminder(&ws.cards[0].items[0], 40000)));
        let mut calls = 0;
        assert!(store
            .process_time(40000, |_, _| {
                calls += 1;
                Ok(())
            })
            .unwrap());
        assert_eq!(calls, 1);
        drop(store);
        let store = fixture.load();
        assert!(!store
            .process_time(200000, |_, _| panic!("duplicate notification"))
            .unwrap());
        assert_eq!(store.read(|ws| badge(ws, 200000).urgent), 1);
        store
            .write(|ws| {
                ws.cards[0].items[0].done = true;
                Ok(())
            })
            .unwrap();
        assert_eq!(store.read(|ws| badge(ws, 200000).urgent), 0);
    }

    #[test]
    fn failed_mutations_do_not_change_memory_or_disk() {
        let fixture = Fixture::new();
        let store = fixture.load();
        let before = serde_json::to_string(&store.snapshot()).unwrap();
        let result: Result<()> = store.write(|ws| {
            ws.cards[0].title = "not saved".into();
            Err(Error::Invalid("test failure".into()))
        });
        assert!(result.is_err());
        assert_eq!(serde_json::to_string(&store.snapshot()).unwrap(), before);
        // Obstruct the temporary file to simulate a disk-write failure.
        fs::create_dir(store.path.with_extension("json.tmp")).unwrap();
        assert!(store
            .write(|ws| {
                ws.cards[0].title = "not saved either".into();
                Ok(())
            })
            .is_err());
        assert_eq!(serde_json::to_string(&store.snapshot()).unwrap(), before);
        let disk: Workspace =
            serde_json::from_str(&fs::read_to_string(&store.path).unwrap()).unwrap();
        assert_eq!(serde_json::to_string(&disk).unwrap(), before);
    }
}
