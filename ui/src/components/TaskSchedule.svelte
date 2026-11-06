<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { Frequency, Item } from '../lib/types';
  import { localInput, nextReset } from '../lib/schedule';
  import { workspace } from '../lib/store.svelte';
  import * as ipc from '../lib/ipc';

  let { item, cardId, onclose }: { item: Item; cardId: string; onclose: () => void } = $props();
  const initial = untrack(() => item.schedule);
  const targetCardId = untrack(() => cardId);
  let due = $state(localInput(initial?.dueAt));
  const presets = ['off', '0', '10', '30', '60', '1440'];
  const initialReminder = initial?.remindBeforeMinutes == null ? 'off' : String(initial.remindBeforeMinutes);
  let reminder = $state(presets.includes(initialReminder) ? initialReminder : 'custom');
  let custom = $state(initial?.remindBeforeMinutes ?? 30);
  let frequency = $state<Frequency | ''>(initial?.repeat?.frequency ?? '');
  let reset = $state(localInput(nextReset(initial)));
  let busy = $state(false);
  let error = $state('');

  onMount(() => {
    // The native date picker temporarily takes focus away from the panel.
    void ipc.suspendAutoCollapse(true);
    return () => { void ipc.suspendAutoCollapse(false); };
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    error = '';
    const dueAt = due ? new Date(due).getTime() : null;
    const nextResetAt = frequency && reset ? new Date(reset).getTime() : null;
    const minutes = !due || reminder === 'off' ? null : reminder === 'custom' ? custom : Number(reminder);
    if (minutes != null && (!Number.isInteger(minutes) || minutes < 0 || minutes > 525600)) {
      error = '提前量请输入 0～525600 的整数分钟'; return;
    }
    if ((dueAt != null && !Number.isFinite(dueAt)) || (nextResetAt != null && !Number.isFinite(nextResetAt))) {
      error = '请填写有效日期'; return;
    }
    if (frequency && (nextResetAt == null || nextResetAt <= Date.now() || (dueAt != null && nextResetAt <= dueAt))) {
      error = '下次重置须晚于现在及本期截止时间'; return;
    }
    busy = true;
    try {
      if (await workspace.saveSchedule(targetCardId, item.id, { dueAt, remindBeforeMinutes: minutes, frequency: frequency || null, nextResetAt })) onclose();
      else error = '未保存，请检查提示后重试';
    } finally { busy = false; }
  }

  function escape(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.stopPropagation(); if (!busy) onclose(); }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form class="editor" onsubmit={save} onkeydown={escape} aria-label="任务时间设置">
  <header><strong>截止与循环</strong><div class="actions"><button type="button" onclick={onclose} disabled={busy}>取消</button><button type="submit" class="save" disabled={busy}>{busy ? '保存中…' : '保存设置'}</button></div></header>
  <div class="body">
  <p class="task">{item.text}</p>
  <div class="fields">
    <label>本期截止时间（可选）<input type="datetime-local" bind:value={due} min="2000-01-01T00:00" max="2099-12-31T23:59" /></label>
    <label>系统提醒<select bind:value={reminder} disabled={!due}>
      <option value="off">不提醒</option><option value="0">到期时</option><option value="10">提前 10 分钟</option><option value="30">提前 30 分钟</option><option value="60">提前 1 小时</option><option value="1440">提前 1 天</option><option value="custom">自定义分钟数</option>
    </select></label>
    {#if due && reminder === 'custom'}<label>提前多少分钟<input type="number" bind:value={custom} min="0" max="525600" step="1" required /></label>{/if}
    <label>自动重置<select bind:value={frequency}><option value="">不循环</option><option value="weekly">每周</option><option value="monthly">每月</option></select></label>
    {#if frequency}<label>下次重置时间<input type="datetime-local" bind:value={reset} min="2000-01-01T00:00" max="2099-12-31T23:59" required /></label>{/if}
  </div>
  <p class="hint">完成后保持勾选，到重置时间才恢复未完成，截止时间同步顺延。每月遇到短月按月底处理。</p>
  <p class="hint">提醒需保持 Jotter 运行（可收起到托盘），检查间隔约 15 秒；退出期间错过的提醒会在重开后补一次。</p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  </div>
</form>

<style>
  .editor { position: absolute; inset: 34px 0 34px 23px; z-index: 10; display: flex; flex-direction: column; padding: 8px 14px; background: var(--paper); }
  header { flex: none; display: flex; align-items: center; justify-content: space-between; gap: 8px; padding-bottom: 6px; border-bottom: 1px solid var(--hairline); }
  .actions { display: flex; gap: 6px; }
  .body { flex: 1; min-height: 0; overflow-y: auto; padding: 0 2px; }
  strong { font-size: 13px; }
  button { padding: 5px 10px; font-size: 12px; background: var(--surface-hover); }
  .task { font-size: 12px; color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin: 6px 0 10px; }
  .fields { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; }
  label { display: flex; flex-direction: column; gap: 5px; font-size: 11px; color: var(--text-dim); }
  input, select { width: 100%; min-width: 0; font: inherit; font-size: 12px; padding: 6px; border: 1px solid var(--hairline-strong); border-radius: 6px; background: var(--surface); color: var(--text); }
  .hint { font-size: 11px; line-height: 1.5; color: var(--text-faint); margin: 10px 0; }
  .error { font-size: 12px; color: var(--danger); }
  .save { color: white; background: var(--accent); }
</style>
