<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import * as ipc from '../lib/ipc';
  import type { SunsetSettings } from '../lib/types';
  import { fetchSunsetForecast, formatSunsetDetails, geocodeLocation } from '../lib/sunset';

  let enabled = $state(false);
  let busy = $state(true);
  let ready = $state(false);
  let error = $state('');
  let status = $state('');
  let notificationBusy = $state(false);
  let notificationError = $state<string | null>(null);
  let notificationStatus = $state('');
  let sunsetEnabled = $state(true);
  let sunsetLocation = $state('浦东新区 · 上海市 · 中国');
  let sunsetSettings = $state<SunsetSettings | null>(null);
  let sunsetBusy = $state(false);
  let sunsetError = $state<string | null>(null);
  let sunsetStatus = $state('');
  let sunsetPreview = $state('');

  async function testNotification() {
    notificationBusy = true;
    notificationStatus = '';
    try {
      await ipc.testNotification();
      notificationError = null;
      notificationStatus = '已提交给 Windows，请检查右下角或通知中心。';
    } catch (err) { notificationError = String(err); }
    finally { notificationBusy = false; }
  }

  async function refresh() {
    busy = true;
    error = '';
    status = '';
    try {
      const [autostart, sunset] = await Promise.all([ipc.getAutostart(), ipc.getSunsetSettings()]);
      enabled = autostart;
      sunsetSettings = sunset;
      sunsetEnabled = sunset.enabled;
      sunsetLocation = sunset.location;
      ready = true;
    } catch (err) {
      ready = false;
      error = `读取失败：${String(err)}`;
    } finally {
      busy = false;
    }
  }

  async function saveSunset() {
    if (sunsetBusy) return;
    sunsetBusy = true;
    sunsetError = null;
    sunsetStatus = '';
    sunsetPreview = '';
    try {
      const current = sunsetSettings;
      if (sunsetEnabled) {
        const place = await geocodeLocation(sunsetLocation);
        const input = {
          enabled: true,
          location: place.name,
          latitude: place.latitude,
          longitude: place.longitude,
          timezone: place.timezone,
        };
        await ipc.setSunsetSettings(input);
        sunsetSettings = { ...input, lastNotifiedDate: null };
        sunsetLocation = place.name;
        sunsetStatus = `已保存：${place.name}。每天本机 16:00 查询并提醒。`;
      } else {
        const input = {
          enabled: false,
          location: sunsetLocation.trim(),
          latitude: current?.latitude ?? null,
          longitude: current?.longitude ?? null,
          timezone: current?.timezone ?? null,
        };
        await ipc.setSunsetSettings(input);
        sunsetSettings = { ...input, lastNotifiedDate: current?.lastNotifiedDate ?? null };
        sunsetStatus = '晚霞提醒已关闭。';
      }
    } catch (err) {
      sunsetError = String(err);
    } finally {
      sunsetBusy = false;
    }
  }

  async function previewSunset() {
    if (sunsetBusy || !sunsetLocation.trim()) return;
    sunsetBusy = true;
    sunsetError = null;
    sunsetStatus = '';
    try {
      const current = sunsetSettings;
      const place = await geocodeLocation(sunsetLocation);
      const forecast = await fetchSunsetForecast({
        enabled: true,
        location: place.name,
        latitude: place.latitude,
        longitude: place.longitude,
        timezone: place.timezone,
        lastNotifiedDate: current?.lastNotifiedDate ?? null,
      });
      sunsetPreview = formatSunsetDetails(forecast);
    } catch (err) {
      sunsetError = String(err);
    } finally {
      sunsetBusy = false;
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
    void ipc.getReminderStatus().then((value) => notificationError = value).catch((err) => notificationError = String(err));
    // This webview stays alive while hidden. Re-read on opening and after
    // returning to this tab, so a stale UI never overwrites external changes.
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let offStatus: (() => void) | undefined;
    void listen<string | null>('reminder-status', (event) => notificationError = event.payload).then((off) => {
      if (disposed) off(); else offStatus = off;
    });
    void listen<boolean>('panel-state', (event) => {
      if (event.payload && !busy) void refresh();
    }).then((off) => {
      if (disposed) off();
      else unlisten = off;
    });
    return () => {
      disposed = true;
      unlisten?.();
      offStatus?.();
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
  <div class="notifications">
    <h2>任务提醒</h2>
    <p>在任务右侧的日历按钮中设置截止时间、提前提醒和循环。</p>
    <p class="hint">保持 Jotter 运行即可提醒；勿扰模式可能让通知只出现在通知中心。没有开启提醒的任务在逾期后角标也会变红。</p>
    <button class="retry" onclick={testNotification} disabled={notificationBusy}>{notificationBusy ? '发送中…' : '发送测试通知'}</button>
    {#if notificationError}<p class="error" role="alert">{notificationError}</p>{/if}
    <p class="status" role="status">{notificationStatus}</p>
  </div>
  <div class="sunset">
    <div class="setting">
      <div class="copy">
        <h2 id="sunset-label">晚霞提示</h2>
        <p id="sunset-description">每天本机下午 4 点查询指定地点的日落和晚霞强度，并发送 Windows 通知。</p>
      </div>
      <button
        class="switch"
        class:on={sunsetEnabled}
        role="switch"
        aria-checked={sunsetEnabled}
        aria-labelledby="sunset-label"
        aria-describedby="sunset-description"
        disabled={sunsetBusy}
        onclick={() => (sunsetEnabled = !sunsetEnabled)}
      ><span></span></button>
    </div>
    <label class="location">
      查询地点
      <input bind:value={sunsetLocation} maxlength="120" placeholder="例如：上海市浦东新区" />
    </label>
    <p class="hint">强度是基于日落前后云量、能见度和降水概率的预测估算，数据来自 Open-Meteo。</p>
    <div class="actions">
      <button class="retry" onclick={previewSunset} disabled={sunsetBusy || !sunsetLocation.trim()}>
        {sunsetBusy ? '查询中…' : '查询今日预测'}
      </button>
      <button class="retry save" onclick={saveSunset} disabled={sunsetBusy || (sunsetEnabled && !sunsetLocation.trim())}>
        {sunsetBusy ? '保存中…' : '保存设置'}
      </button>
    </div>
    {#if sunsetPreview}<p class="preview" role="status">{sunsetPreview}</p>{/if}
    {#if sunsetError}<p class="error" role="alert">{sunsetError}</p>{/if}
    <p class="status" role="status">{sunsetStatus}</p>
  </div>
</section>

<style>
  .notifications, .sunset { border-top: 1px solid var(--hairline); padding-top: 14px; margin-top: 14px; }
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

  .sunset .location {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-top: 12px;
    font-size: 11px;
    color: var(--text-dim);
  }

  .location input {
    min-height: 30px;
    padding: 5px 8px;
    border: 1px solid var(--hairline-strong);
    border-radius: 8px;
    background: var(--surface-sunken);
    user-select: text;
  }

  .actions {
    display: flex;
    gap: 7px;
  }

  .save {
    color: var(--accent);
  }

  .preview {
    margin-top: 10px;
    padding: 7px 8px;
    border-radius: 8px;
    background: var(--surface-sunken);
    color: var(--text-dim);
  }
</style>
