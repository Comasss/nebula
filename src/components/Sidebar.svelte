<script>
  import { invoke } from '@tauri-apps/api/core';
  import Icon from './Icon.svelte';
  import SpeedGraph from './SpeedGraph.svelte';
  import { ui } from '../lib/state.svelte.js';
  import { speed, bytes } from '../lib/format.js';
  import { usableDebrid, debridLabel } from '../lib/debrid.js';

  const ALL = [
    { id: 'search', label: 'Buscar', icon: 'search', feature: 'indexers' },
    { id: 'downloads', label: 'Descargas', icon: 'download', feature: 'nntp' },
    { id: 'history', label: 'Historial', icon: 'history', feature: 'nntp' },
    { id: 'indexers', label: 'Indexadores', icon: 'layers', feature: 'indexers' },
    { id: 'servers', label: 'Servidores', icon: 'server', feature: 'nntp' },
    { id: 'debrid', label: 'Debrid', icon: 'cloud', feature: 'nntp' },
    { id: 'settings', label: 'Configuración', icon: 'settings' },
    { id: 'logs', label: 'Registro', icon: 'logs' },
  ];
  const items = $derived(
    ALL.filter(
      (i) => !i.feature || (i.feature === 'nntp' ? ui.settings?.featureNntp !== false : ui.settings?.featureIndexers)
    )
  );
  const ITEM_H = 42;
  const GAP = 4;

  const active = $derived(Math.max(0, items.findIndex((i) => i.id === ui.view)));
  const queueCount = $derived(ui.queue.jobs.length);
  const failedCount = $derived(ui.history.filter((h) => h.status === 'failed').length);
  const errorLogs = $derived(ui.logs.filter((l) => l.level === 'error').length);
  const debridJob = $derived(ui.queue.jobs.find((j) => j.debrid && j.status === 'downloading'));
  const serverState = $derived.by(() => {
    const s = ui.queue.servers;
    const debrid = usableDebrid(ui.settings);
    if (debridJob && !ui.queue.activeConns) return { cls: 'active', text: `Descargando vía ${debridJob.debrid.provider}` };
    if (!ui.settings?.servers?.length) return debrid.length ? { cls: 'ok', text: `Listo · ${debridLabel(debrid[0])}` } : { cls: 'err', text: 'Sin servidores' };
    if (!s.length) return { cls: '', text: 'Servidores desactivados' };
    if (s.some((x) => x.state === 'auth')) return { cls: 'err', text: 'Error de autenticación' };
    if (s.every((x) => x.state === 'down')) return { cls: 'err', text: 'Sin conexión' };
    if (s.some((x) => x.state === 'down' || x.state === 'warn')) return { cls: 'warn', text: 'Conexión inestable' };
    const conns = s.reduce((a, x) => a + x.connected, 0);
    if (s.some((x) => x.state === 'active')) return { cls: 'active', text: `${ui.queue.activeConns} conexiones activas` };
    if (conns) return { cls: 'idle', text: `${conns} conexiones abiertas` };
    return { cls: 'ok', text: 'Listo' };
  });

  function togglePause() {
    invoke('set_paused', { paused: !ui.queue.paused });
  }
</script>

<nav class="sidebar">
  <div class="nav" style="--h:{ITEM_H}px; --gap:{GAP}px">
    <div class="pill" style="transform: translateY({active * (ITEM_H + GAP)}px)"></div>
    {#each items as it (it.id)}
      <button class="item" class:active={ui.view === it.id} onclick={() => (ui.view = it.id)}>
        <Icon name={it.icon} size={18} />
        <span class="label">{it.label}</span>
        {#if it.id === 'downloads' && queueCount}
          <span class="badge">{queueCount}</span>
        {:else if it.id === 'history' && failedCount}
          <span class="badge err">{failedCount}</span>
        {:else if it.id === 'logs' && errorLogs}
          <span class="badge err small"></span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="grow"></div>

  {#if ui.settings?.featureNntp !== false}
  <div class="speed-card card">
    <div class="sc-head">
      <div>
        <div class="sc-label">Velocidad</div>
        <div class="sc-speed">{ui.queue.paused ? '—' : speed(ui.queue.speed)}</div>
      </div>
      <button
        class="pause-btn"
        class:paused={ui.queue.paused}
        onclick={togglePause}
        title={ui.queue.paused ? 'Reanudar todas las descargas' : 'Pausar todas las descargas'}>
        <Icon name={ui.queue.paused ? 'play' : 'pause'} size={15} stroke={2.4} />
      </button>
    </div>
    <div class="sc-graph">
      <SpeedGraph data={ui.speedHistory.slice(-60)} height={42} id="side" />
    </div>
    <div class="sc-foot">
      <span class="dot {serverState.cls}"></span>
      <span class="truncate">{serverState.text}</span>
      {#if ui.queue.remaining > 0}
        <span class="spacer"></span>
        <span class="muted nowrap">{bytes(ui.queue.remaining)}</span>
      {/if}
    </div>
  </div>
  {/if}
</nav>

<style>
  .sidebar {
    width: 236px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    padding: 14px 12px 14px;
    background: var(--bg);
    border-right: 1px solid rgba(255, 255, 255, 0.035);
  }
  .nav { position: relative; display: flex; flex-direction: column; gap: var(--gap); }
  .pill {
    position: absolute;
    left: 0; right: 0; top: 0;
    height: var(--h);
    border-radius: 11px;
    background: linear-gradient(90deg, rgba(var(--accent-rgb), 0.2), rgba(var(--accent-rgb), 0.06));
    border: 1px solid rgba(var(--accent-rgb), 0.28);
    transition: transform 0.42s var(--ease);
    pointer-events: none;
  }
  .pill::before {
    content: '';
    position: absolute;
    left: -1px; top: 10px; bottom: 10px;
    width: 3px;
    border-radius: 3px;
    background: var(--grad-diag);
    box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.9);
  }
  .item {
    position: relative;
    height: var(--h);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 14px;
    border: none;
    border-radius: 11px;
    background: transparent;
    color: var(--text-2);
    font-weight: 550;
    cursor: pointer;
    text-align: left;
    transition: color 0.2s, background 0.2s;
  }
  .item:hover:not(.active) { background: rgba(255, 255, 255, 0.035); color: var(--text); }
  .item.active { color: var(--text); }
  .item.active :global(svg) { color: var(--accent-2); }
  .item :global(svg) { transition: color 0.25s, transform 0.25s var(--ease); }
  .item:hover :global(svg) { transform: scale(1.08); }
  .label { flex: 1; }
  .badge {
    min-width: 20px; height: 20px; padding: 0 6px;
    border-radius: 999px;
    display: grid; place-items: center;
    font-size: 11px; font-weight: 700;
    background: rgba(var(--accent-rgb), 0.22);
    color: var(--text);
    animation: badge-in 0.35s var(--ease);
  }
  .badge.err { background: rgba(248, 113, 113, 0.2); color: #fca5a5; }
  .badge.small { min-width: 8px; width: 8px; height: 8px; padding: 0; background: var(--err); }
  @keyframes badge-in { from { transform: scale(0.4); opacity: 0; } }
  .grow { flex: 1; }

  .speed-card { padding: 14px 14px 12px; overflow: hidden; }
  .sc-head { display: flex; align-items: flex-start; justify-content: space-between; }
  .sc-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 650; }
  .sc-speed { font-family: var(--font-display); font-size: 20px; font-weight: 650; font-variant-numeric: tabular-nums; margin-top: 2px; }
  .pause-btn {
    width: 34px; height: 34px;
    border-radius: 50%;
    border: 1px solid var(--border-2);
    background: var(--panel-2);
    color: var(--text);
    display: grid; place-items: center;
    cursor: pointer;
    transition: all 0.2s var(--ease);
  }
  .pause-btn:hover { background: var(--panel-3); transform: scale(1.06); }
  .pause-btn.paused { background: var(--grad-diag); border-color: transparent; box-shadow: 0 4px 16px -4px rgba(var(--accent-rgb), 0.9); }
  .sc-graph { margin: 10px -14px 8px; }
  .sc-foot { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-2); min-width: 0; }
  .nowrap { white-space: nowrap; flex-shrink: 0; }
</style>
