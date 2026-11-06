import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';

// Exercise the actual UI helpers without adding a browser-test dependency.
const source = readFileSync(new URL('../ui/src/lib/schedule.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { nextReset, isUrgent, localInput } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);

test('monthly labels retain the original day after a short month', () => {
  const repeat = { frequency: 'monthly', resetAnchor: new Date(2024, 0, 31, 9).getTime(), occurrence: 1 };
  assert.equal(localInput(nextReset({ repeat })), '2024-02-29T09:00');
  repeat.occurrence = 2;
  assert.equal(localInput(nextReset({ repeat })), '2024-03-31T09:00');
});

test('reminder threshold, disabled reminders and completion control urgency', () => {
  const item = { done: false, schedule: { dueAt: 120000, remindBeforeMinutes: 1 } };
  assert.equal(isUrgent(item, 59999), false);
  assert.equal(isUrgent(item, 60000), true);
  item.done = true;
  assert.equal(isUrgent(item, 300000), false);
  item.done = false;
  item.schedule.remindBeforeMinutes = null;
  assert.equal(isUrgent(item, 119999), false);
  assert.equal(isUrgent(item, 120000), true);
});

test('DST gap uses the first valid minute, matching Rust recurrence', () => {
  const previous = process.env.TZ;
  process.env.TZ = 'America/New_York';
  try {
    const repeat = { frequency: 'weekly', resetAnchor: new Date(2026, 2, 1, 2, 30).getTime(), occurrence: 1 };
    assert.equal(localInput(nextReset({ repeat })), '2026-03-08T03:00');
  } finally {
    if (previous === undefined) delete process.env.TZ;
    else process.env.TZ = previous;
  }
});
