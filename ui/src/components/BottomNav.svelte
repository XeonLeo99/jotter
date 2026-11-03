<!--
  Tab switcher at the foot of the notebook. The active tab lives in the parent, which owns which content
  the panel shows; this component only renders the affordance.
-->
<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    active = $bindable('notes'),
  }: { active?: 'notes' | 'skills' | 'settings' } = $props();
</script>

<nav class="nav">
  <button
    class="tab"
    class:on={active === 'notes'}
    onclick={() => (active = 'notes')}
    aria-pressed={active === 'notes'}
    title="本子 — 记录问题"
  >
    <Icon name="book" size={14} />
    <span>本子</span>
  </button>
  <button
    class="tab"
    class:on={active === 'skills'}
    onclick={() => (active = 'skills')}
    aria-pressed={active === 'skills'}
    title="技能 — 管理可复用的提示词"
  >
    <Icon name="sparkle" size={14} />
    <span>技能</span>
  </button>
  <button
    class="tab"
    class:on={active === 'settings'}
    onclick={() => (active = 'settings')}
    aria-pressed={active === 'settings'}
    title="设置"
  >
    <Icon name="settings" size={14} />
    <span>设置</span>
  </button>
</nav>

<style>
  .nav {
    display: flex;
    flex: none;
    border-top: 1px solid var(--hairline);
    padding: 0 10px;
    gap: 4px;
  }

  .tab {
    position: relative;
    height: 34px;
    padding: 0 14px;
    gap: 6px;
    font-size: 12px;
    color: var(--text-faint);
  }

  .tab:hover {
    color: var(--text-dim);
  }

  .tab.on {
    color: var(--text);
    font-weight: 600;
  }

  /* An underline rather than a filled pill keeps the bar quiet next to the
     capture line directly above it. */
  .tab.on::after {
    content: '';
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 0;
    height: 2px;
    border-radius: 2px 2px 0 0;
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
  }
</style>
