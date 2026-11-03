<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import * as ipc from '../lib/ipc';

  let enabled = $state(false);
  let busy = $state(true);
  let ready = $state(false);
  let error = $state('');
  let status = $state('');

  async function refresh() {
    busy = true;
    error = '';
    status = '';
    try {
      enabled = await ipc.getAutostart();
      ready = true;
    } catch (err) {
      ready = false;
      error = `读取失败：${String(err)}`;
    } finally {
      busy = false;
    }
  }

  async function toggle() {
    if (busy || !ready) return;
    busy = true;
    error = '';
    status = '';
    const next = !enabled;
    try {
      await ipc.setAutostart(next);
      enabled = next;
      status = next ? '已开启，下次登录 Windows 时生效' : '已关闭开机自启';
    } catch (err) {
      error = `保存失败：${String(err)}`;
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void refresh();
    // This webview stays alive while hidden. Re-read on opening and after
    // returning to this tab, so a stale UI never overwrites external changes.
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<boolean>('panel-state', (event) => {
      if (event.payload && !busy) void refresh();
    }).then((off) => {
      if (disposed) off();
      else unlisten = off;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<section class="settings" aria-label="设置">
  <div class="setting">
    <div class="copy">
      <h2 id="autostart-label">开机自启</h2>
      <p id="autostart-description">登录 Windows 后自动显示桌面宠物，无需管理员权限。</p>
    </div>
    <button
      class="switch"
      class:on={enabled}
      role="switch"
      aria-checked={enabled}
      aria-labelledby="autostart-label"
      aria-describedby="autostart-description"
      disabled={busy || !ready}
      onclick={toggle}
    ><span></span></button>
  </div>
  <p class="hint">默认关闭，仅对当前账号生效。便携版移动或改名后，请在新位置重新开启；删除程序前请先关闭。</p>
  <p class="hint">若曾在任务管理器中禁用 Jotter，请在那里重新启用。</p>
  {#if error}
    <p class="error" role="alert">{error}</p>
    <button class="retry" onclick={refresh} disabled={busy}>重新读取</button>
  {:else}
    <p class="status" role="status">{busy ? '正在处理…' : status}</p>
  {/if}
</section>

<style>
  .settings {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 14px 16px;
  }

  .setting {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .copy {
    flex: 1;
  }

  h2 {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
  }

  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-dim);
  }

  .hint {
    margin-top: 12px;
    font-size: 11px;
    color: var(--text-faint);
  }

  .switch {
    flex: none;
    width: 34px;
    height: 20px;
    padding: 3px;
    justify-content: flex-start;
    border-radius: 12px;
    background: var(--text-faint);
  }

  .switch.on {
    background: var(--accent);
  }

  .switch span {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--paper);
  }

  .switch.on span {
    transform: translateX(14px);
  }

  .status, .error {
    margin-top: 12px;
    font-size: 11px;
  }

  .error {
    color: var(--danger);
    overflow-wrap: anywhere;
  }

  .retry {
    margin-top: 6px;
    padding: 4px 8px;
    background: var(--surface-hover);
    font-size: 12px;
  }
</style>
