<script>
  // Navegación de la versión móvil: barra inferior con las vistas principales y un menú "Más"
  // con el resto (las mismas vistas y contadores que la barra lateral del escritorio).
  import { t, tn } from '../lib/i18n.svelte.js';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import SpeedGraph from './SpeedGraph.svelte';
  import { ui, d } from '../lib/state.svelte.js';
  import { speed, bytes } from '../lib/format.js';

  const ALL = [
    { id: 'downloads', icon: 'download', feature: 'nntp' },
    { id: 'search', icon: 'search', feature: 'indexers' },
    { id: 'history', icon: 'history', feature: 'nntp' },
    { id: 'stats', icon: 'chart' },
    { id: 'indexers', icon: 'layers', feature: 'indexers' },
    { id: 'servers', icon: 'server', feature: 'nntp' },
    { id: 'debrid', icon: 'cloud', feature: 'nntp' },
    { id: 'settings', icon: 'settings' },
    { id: 'logs', icon: 'logs' },
  ];
  const items = $derived(
    ALL.filter((i) => !i.feature || (i.feature === 'nntp' ? ui.settings?.featureNntp !== false : ui.settings?.featureIndexers))
  );
  const main = $derived(items.slice(0, 4));
  const rest = $derived(items.slice(4));
  const inRest = $derived(rest.some((i) => i.id === ui.view));

  const queueCount = $derived(ui.queue.jobs.length);
  const failedCount = $derived(ui.history.filter((h) => h.status === 'failed').length);
  const errorLogs = $derived(ui.logs.filter((l) => l.level === 'error').length);

  function go(id) {
    ui.view = id;
    ui.moreOpen = false;
  }
</script>

{#snippet badge(id)}
  {#if id === 'downloads' && queueCount}
    <span class="badge">{queueCount}</span>
  {:else if id === 'history' && failedCount}
    <span class="badge err">{failedCount}</span>
  {:else if id === 'logs' && errorLogs}
    <span class="badge err small"></span>
  {/if}
{/snippet}

<nav class="mnav">
  {#each main as it (it.id)}
    <button class="tab" class:active={ui.view === it.id && !ui.moreOpen} onclick={() => go(it.id)}>
      <span class="ic"><Icon name={it.icon} size={20} />{@render badge(it.id)}</span>
      <span class="label">{t(`nav.${it.id}`)}</span>
    </button>
  {/each}
  {#if rest.length}
    <button class="tab" class:active={inRest || ui.moreOpen} onclick={() => (ui.moreOpen = !ui.moreOpen)}>
      <span class="ic">
        <Icon name="more" size={20} stroke={3} />
        {#if rest.some((i) => i.id === 'logs') && errorLogs}<span class="badge err small"></span>{/if}
      </span>
      <span class="label">{t('mobile.more')}</span>
    </button>
  {/if}
</nav>

{#if ui.moreOpen}
  <div class="backdrop" transition:fade={{ duration: d(160) }} onclick={() => (ui.moreOpen = false)} role="presentation"></div>
  <div class="sheet" transition:fly={{ y: 260, duration: d(280), easing: cubicOut }}>
    <div class="grip"></div>
    <div class="grid">
      {#each rest as it (it.id)}
        <button class="tile" class:active={ui.view === it.id} onclick={() => go(it.id)}>
          <span class="t-icon"><Icon name={it.icon} size={20} />{@render badge(it.id)}</span>
          <span class="t-label">{t(`nav.${it.id}`)}</span>
        </button>
      {/each}
    </div>
    {#if ui.settings?.featureNntp !== false}
      <div class="speed card">
        <div class="sp-top">
          <div>
            <div class="sp-label">{t('sidebar.speed')}</div>
            <div class="sp-value">{ui.queue.paused ? '—' : speed(ui.queue.speed)}</div>
          </div>
          {#if ui.queue.remaining > 0}<span class="muted">{bytes(ui.queue.remaining)}</span>{/if}
        </div>
        <div class="sp-graph"><SpeedGraph data={ui.speedHistory.slice(-60)} height={40} id="more" /></div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .mnav {
    flex-shrink: 0;
    display: flex;
    height: 62px;
    padding: 0 4px;
    background: var(--bg);
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    position: relative;
    z-index: 60;
  }
  .tab {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: -0.01em;
    cursor: pointer;
    transition: color 0.2s;
    -webkit-tap-highlight-color: transparent;
  }
  .ic {
    position: relative;
    width: 56px; height: 30px;
    border-radius: 999px;
    display: grid; place-items: center;
    transition: background 0.3s var(--ease), color 0.2s;
  }
  .tab.active { color: var(--text); }
  .tab.active .ic {
    background: linear-gradient(90deg, rgba(var(--accent-rgb), 0.26), rgba(var(--accent-rgb), 0.1));
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.3);
    color: var(--accent-2);
  }
  .tab:active .ic { transform: scale(0.94); }
  .label { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge {
    position: absolute; top: -3px; right: 6px;
    min-width: 17px; height: 17px; padding: 0 5px;
    border-radius: 999px;
    display: grid; place-items: center;
    font-size: 10px; font-weight: 700; color: var(--text);
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--bg);
  }
  .badge.err { background: #dc5050; }
  .badge.small { min-width: 9px; width: 9px; height: 9px; padding: 0; top: 0; right: 14px; background: var(--err); }

  .backdrop { position: fixed; inset: 0 0 62px 0; z-index: 55; background: rgba(5, 6, 10, 0.55); backdrop-filter: blur(4px); }
  .sheet {
    position: fixed; left: 0; right: 0; bottom: 62px; z-index: 56;
    padding: 8px 14px 16px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.025), transparent 120px), var(--panel);
    border: 1px solid var(--border-2); border-bottom: none;
    border-radius: 20px 20px 0 0;
    box-shadow: var(--shadow);
  }
  .grip { width: 38px; height: 4px; border-radius: 4px; background: var(--panel-4); margin: 2px auto 14px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
  .tile {
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    padding: 14px 6px 12px;
    border-radius: 14px;
    border: 1px solid var(--border);
    background: var(--panel-2);
    color: var(--text-2);
    font-size: 12.5px; font-weight: 600;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s, color 0.2s;
  }
  .tile:active { transform: scale(0.97); }
  .tile.active { border-color: rgba(var(--accent-rgb), 0.45); background: rgba(var(--accent-rgb), 0.12); color: var(--text); }
  .t-icon {
    position: relative;
    width: 40px; height: 40px; border-radius: 12px;
    display: grid; place-items: center;
    background: var(--panel-3); color: var(--accent-2);
  }
  .t-icon .badge { top: -5px; right: -7px; }
  .t-icon .badge.small { top: -2px; right: -2px; }
  .t-label { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .speed { margin-top: 12px; padding: 12px 14px 8px; overflow: hidden; }
  .sp-top { display: flex; justify-content: space-between; align-items: flex-start; font-size: 12px; }
  .sp-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 650; }
  .sp-value { font-family: var(--font-display); font-size: 19px; font-weight: 650; font-variant-numeric: tabular-nums; margin-top: 2px; }
  .sp-graph { margin: 6px -14px 0; }
</style>
