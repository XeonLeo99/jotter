<!--
  Reusable prompt / context snippets: a name plus a body of text (server info,
  credentials, conventions) that can be copied back into a fresh session.

  Three views over the same list: the name list, a read-only detail with a
  one-click copy, and an inline editor for create / edit. Escape unwinds one
  layer at a time (detail -> list, editor -> where it came from) before falling
  through to the panel-wide handler that closes the notebook.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import * as ipc from '../lib/ipc';
  import { toasts, workspace } from '../lib/store.svelte';
  import type { Skill } from '../lib/types';

  type Mode =
    | { kind: 'list' }
    | { kind: 'detail'; id: string }
    | { kind: 'edit'; id: string | null };

  let mode = $state<Mode>({ kind: 'list' });
  let query = $state('');
  let name = $state('');
  let content = $state('');
  let confirmingId = $state<string | null>(null);

  // Display order is alphabetical, so the list reads like a small library; the
  // storage order stays insertion order and does not churn while editing.
  const sorted = $derived([...workspace.skills].sort((a, b) => a.name.localeCompare(b.name, 'zh')));
  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sorted;
    return sorted.filter((s) => `${s.name}\n${s.content}`.toLowerCase().includes(q));
  });
  const openSkill = $derived.by(() => {
    const m = mode;
    if (m.kind === 'detail') return workspace.skills.find((s) => s.id === m.id) ?? null;
    if (m.kind === 'edit' && m.id !== null) return workspace.skills.find((s) => s.id === m.id) ?? null;
    return null;
  });

  onMount(() => {
    // Capture phase: runs before the panel's bubble-phase Escape handler, so a
    // detail/editor view can consume Escape for itself rather than closing the
    // whole notebook.
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      if (mode.kind === 'detail') {
        event.stopPropagation();
        mode = { kind: 'list' };
      } else if (mode.kind === 'edit') {
        event.stopPropagation();
        cancelEdit();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });

  function openDetail(id: string) {
    confirmingId = null;
    mode = { kind: 'detail', id };
  }

  function startCreate() {
    confirmingId = null;
    name = '';
    content = '';
    mode = { kind: 'edit', id: null };
  }

  function startEdit(skill: Skill) {
    confirmingId = null;
    name = skill.name;
    content = skill.content;
    mode = { kind: 'edit', id: skill.id };
  }

  function cancelEdit() {
    if (mode.kind !== 'edit') return;
    mode = mode.id !== null ? { kind: 'detail', id: mode.id } : { kind: 'list' };
  }

  async function save() {
    const value = name.trim();
    if (!value || mode.kind !== 'edit') return;
    if (mode.id !== null) {
      const id = mode.id;
      const ok = await workspace.updateSkill(id, value, content);
      if (ok) mode = { kind: 'detail', id };
    } else {
      const ok = await workspace.addSkill(value, content);
      if (ok) {
        toasts.show(`已添加技能「${value}」`);
        mode = { kind: 'list' };
      }
    }
  }

  async function copySkill(skill: Skill) {
    try {
      await ipc.copyText(skill.content);
      toasts.show(`已复制「${skill.name}」`);
    } catch (err) {
      toasts.show(`复制失败：${String(err)}`, 'error');
    }
  }

  async function removeSkill(id: string) {
    if (confirmingId !== id) {
      confirmingId = id;
      return;
    }
    confirmingId = null;
    await workspace.removeSkill(id);
    if (mode.kind === 'detail' && mode.id === id) mode = { kind: 'list' };
  }

  function onNameKey(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.isComposing) {
      event.preventDefault();
      void save();
    }
  }
</script>

<div class="skills">
  {#if mode.kind === 'edit'}
    <div class="editor">
      <header class="bar">
        <button class="icon" onclick={cancelEdit} title="取消（Esc）" aria-label="取消">
          <Icon name="arrow-left" />
        </button>
        <span class="title">{mode.id !== null ? '编辑技能' : '新建技能'}</span>
        <span class="spacer"></span>
        <button class="act primary" onclick={save} disabled={name.trim().length === 0}>
          保存
        </button>
      </header>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="name-input"
        bind:value={name}
        maxlength="60"
        autofocus
        onkeydown={onNameKey}
        placeholder="技能名称，如：xxx项目生成部署"
        aria-label="技能名称"
      />
      <textarea
        class="content-input"
        bind:value={content}
        maxlength="50000"
        spellcheck="false"
        placeholder="技能详情：服务器 IP、账号密码、部署规范……"
        aria-label="技能详情"
      ></textarea>
    </div>
  {:else if mode.kind === 'detail' && openSkill}
    <div class="detail">
      <header class="bar">
        <button class="icon" onclick={() => (mode = { kind: 'list' })} title="返回（Esc）" aria-label="返回">
          <Icon name="arrow-left" />
        </button>
        <span class="title">{openSkill.name}</span>
        <span class="spacer"></span>
        <button class="icon" onclick={() => startEdit(openSkill)} title="编辑" aria-label="编辑">
          <Icon name="pen" />
        </button>
        <button
          class="icon danger"
          class:confirming={confirmingId === openSkill.id}
          onclick={() => removeSkill(openSkill.id)}
          title={confirmingId === openSkill.id ? '再点一次确认删除' : '删除技能'}
          aria-label="删除技能"
        >
          <Icon name="trash" />
        </button>
        <button class="act primary" onclick={() => copySkill(openSkill)} title="复制详情到剪贴板">
          复制
        </button>
      </header>
      <div class="detail-text">{openSkill.content}</div>
    </div>
  {:else}
    <div class="list-view">
      <header class="bar">
        <span class="title">
          <Icon name="sparkle" size={13} />
          技能
        </span>
        <span class="count">{workspace.skills.length}</span>
        <span class="spacer"></span>
        <button class="icon" onclick={startCreate} title="添加技能" aria-label="添加技能">
          <Icon name="plus" size={14} />
        </button>
      </header>

      <label class="search">
        <Icon name="search" size={13} />
        <input bind:value={query} placeholder="搜索技能…" aria-label="搜索技能" />
      </label>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="list" onpointerleave={() => (confirmingId = null)}>
        {#if filtered.length === 0}
          <div class="empty">
            {#if workspace.skills.length === 0}
              <span class="glyph"><Icon name="sparkle" size={19} /></span>
              还没有技能<br />
              <em>把服务器信息、规范封装成提示词，开新会话一键复制</em>
              <button class="cta" onclick={startCreate}>添加技能</button>
            {:else}
              没有匹配的技能
            {/if}
          </div>
        {:else}
          {#each filtered as skill (skill.id)}
            <div class="row">
              <button class="row-title" onclick={() => openDetail(skill.id)} title="查看详情">
                <Icon name="sparkle" size={12} />
                <span>{skill.name}</span>
              </button>
              <button
                class="kill"
                onclick={() => copySkill(skill)}
                title="复制详情"
                aria-label={`复制「${skill.name}」`}
              >
                <Icon name="copy" size={12} />
              </button>
              <button
                class="kill"
                class:confirming={confirmingId === skill.id}
                onclick={() => removeSkill(skill.id)}
                title={confirmingId === skill.id ? '再点一次确认删除' : '删除技能'}
                aria-label={`删除「${skill.name}」`}
              >
                <Icon name={confirmingId === skill.id ? 'trash' : 'x'} size={12} />
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .skills {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    margin: 0 8px;
    padding-bottom: 8px;
  }

  .list-view,
  .detail,
  .editor {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    height: 32px;
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 700;
    color: var(--text-dim);
  }

  .count {
    font-size: 10.5px;
    font-weight: 700;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }

  .spacer {
    flex: 1;
  }

  .icon {
    width: 24px;
    height: 24px;
    flex: none;
    color: var(--text-faint);
  }

  .icon:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .icon.danger:hover,
  .icon.confirming {
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 14%, transparent);
  }

  .act {
    height: 26px;
    padding: 0 10px;
    flex: none;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .act:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }

  .act.primary {
    background: linear-gradient(140deg, var(--accent), var(--accent-2));
    color: #fff;
  }

  .act.primary:disabled {
    opacity: 0.4;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    height: 28px;
    margin-bottom: 8px;
    padding: 0 10px;
    border-radius: var(--r-pill);
    background: var(--surface-sunken);
    color: var(--text-faint);
  }

  .search:focus-within {
    background: var(--surface);
    box-shadow: inset 0 0 0 1.5px color-mix(in oklab, var(--accent) 55%, transparent);
    color: var(--text);
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 28px;
    font-size: 12px;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 4px 4px 8px;
    border-radius: var(--r-card);
    transition: background-color var(--dur-fast) var(--ease);
  }

  .row:hover {
    background: var(--surface-sunken);
  }

  .row-title {
    flex: 1;
    min-width: 0;
    height: 24px;
    justify-content: flex-start;
    text-align: left;
    gap: 7px;
    font-size: 12.5px;
    color: var(--text);
  }

  .row-title span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-title:active {
    transform: none;
  }

  .kill {
    width: 20px;
    height: 20px;
    flex: none;
    color: var(--text-faint);
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease);
  }

  .row:hover .kill {
    opacity: 1;
  }

  .kill:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .kill.confirming {
    opacity: 1;
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 14%, transparent);
  }

  .empty {
    margin: auto;
    text-align: center;
    font-size: 12px;
    line-height: 1.9;
    color: var(--text-faint);
  }

  .empty em {
    display: block;
    font-style: normal;
    font-size: 11px;
    opacity: 0.8;
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    margin: 0 auto 8px;
    border-radius: var(--r-card);
    background: var(--surface-sunken);
    color: var(--text-faint);
  }

  .cta {
    height: 28px;
    padding: 0 12px;
    margin-top: 8px;
    font-size: 12px;
    color: #fff;
    background: linear-gradient(140deg, var(--accent), var(--accent-2));
  }

  .detail-text {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin-top: 2px;
    padding: 9px 10px;
    border-radius: var(--r-card);
    background: var(--surface-sunken);
    font-size: 12px;
    line-height: 1.6;
    color: var(--text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .editor {
    gap: 8px;
  }

  .name-input {
    flex: none;
    height: 30px;
    padding: 0 10px;
    border-radius: var(--r-pill);
    background: var(--surface-sunken);
    font-size: 12.5px;
    font-weight: 600;
  }

  .name-input:focus,
  .content-input:focus {
    background: var(--surface);
    box-shadow: inset 0 0 0 1.5px color-mix(in oklab, var(--accent) 55%, transparent);
  }

  .content-input {
    flex: 1;
    min-height: 0;
    resize: none;
    padding: 9px 10px;
    border-radius: var(--r-card);
    background: var(--surface-sunken);
    font-size: 12.5px;
    line-height: 1.6;
  }
</style>
