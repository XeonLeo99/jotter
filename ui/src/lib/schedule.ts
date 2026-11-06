import type { Item, Schedule } from './types';

export function isUrgent(item: Item, now = Date.now()): boolean {
  const s = item.schedule;
  return !item.done && s?.dueAt != null && now >= s.dueAt - (s.remindBeforeMinutes ?? 0) * 60000;
}

export function nextReset(schedule?: Schedule): number | null {
  const r = schedule?.repeat;
  if (!r) return null;
  const d = new Date(r.resetAnchor);
  const hour = d.getHours();
  const minute = d.getMinutes();
  if (r.frequency === 'weekly') d.setDate(d.getDate() + r.occurrence * 7);
  else {
    const day = d.getDate();
    d.setDate(1);
    d.setMonth(d.getMonth() + r.occurrence);
    const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
    d.setDate(Math.min(day, last));
  }
  // Rust chooses the first valid minute during a spring-forward gap. JS Date
  // normally preserves minutes (02:30 -> 03:30); match the native 03:00 result.
  const desired = Date.UTC(d.getFullYear(), d.getMonth(), d.getDate(), hour, minute);
  for (let n = 0; n < 180; n++) {
    const previous = new Date(d.getTime() - 60000);
    const wall = Date.UTC(previous.getFullYear(), previous.getMonth(), previous.getDate(), previous.getHours(), previous.getMinutes());
    if (wall < desired) break;
    d.setTime(previous.getTime());
  }
  return d.getTime();
}

export function localInput(ms: number | null | undefined): string {
  if (ms == null) return '';
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function scheduleLabel(item: Item, now: number): string {
  const s = item.schedule;
  if (!s) return '';
  const parts: string[] = [];
  if (s.dueAt != null) {
    const prefix = item.done ? '截止' : s.dueAt <= now ? '已逾期' : isUrgent(item, now) ? '即将到期' : '截止';
    parts.push(`${prefix} ${new Date(s.dueAt).toLocaleString([], { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' })}`);
  }
  if (s.repeat) parts.push(s.repeat.frequency === 'weekly' ? '每周重置' : '每月重置');
  return parts.join(' · ');
}
