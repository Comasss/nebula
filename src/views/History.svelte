<script>
  import { invoke } from '@tauri-apps/api/core';
  import { fly, fade, slide } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import Icon from '../components/Icon.svelte';
  import { ui, ask, toast, refreshHistory, d } from '../lib/state.svelte.js';
  import * as f from '../lib/format.js';

  let filter = $state('all');
  let search = $state('');
  let open = $state(null);

  const counts = $derived({
    all: ui.history.length,
    completed: ui.history.filter((h) => h.status === 'completed').length,
    failed: ui.history.filter((h) => h.status === 'failed').length,
  });
  const items = $derived(
    ui.history.filter(
      (h) => (filter === 'all' || h.status === filter) && (!search || h.name.toLowerCase().includes(search.toLowerCase()))
    )
  );
  const totalSize = $derived(ui.history.filter((h) => h.status === 'completed').reduce((a, h) => a + h.size, 0));

  async function retry(h) {
    try {
      await invoke('retry_history', { id: h.id });
      toast('info', `Reintentando: ${h.name}`, 3000);
      await refreshHistory();
      ui.view = 'downloads';
    } catch (e) {
      toast('error', String(e));
    }
  }

  async function retryExtract(h) {
    const r = await ask({
      title: 'Reintentar extracción',
      message: h.error,
      confirmLabel: 'Extraer',
      icon: 'key',
      input: { label: 'Contraseña', placeholder: 'Déjala vacía si no tiene', password: true },
      value: h.password ?? '',
    });
    if (!r.ok) return;
    try {
      await invoke('retry_extract', { id: h.id, password: r.value || null });
      await refreshHistory();
      ui.view = 'downloads';
    } catch (e) {
      toast('error', String(e));
    }
  }

  async function remove(h) {
    let deleteFiles = false;
    if (h.status === 'failed') {
      const r = await ask({
        title: 'Eliminar del historial',
        message: h.name,
        confirmLabel: 'Eliminar',
        danger: true,
        icon: 'trash',
        checkbox: 'Borrar también los archivos temporales',
        checkedDefault: true,
      });
      if (!r.ok) return;
      deleteFiles = r.checked;
    }
    await invoke('remove_history', { id: h.id, deleteFiles });
    await refreshHistory();
  }

  async function clear(onlyCompleted) {
    const r = await ask({
      title: onlyCompleted ? '¿Limpiar completados?' : '¿Vaciar el historial?',
      message: 'Los archivos descargados no se borran, solo la lista.',
      confirmLabel: 'Limpiar',
      danger: true,
      icon: 'trash',
    });
    if (!r.ok) return;
    await invoke('clear_history', { onlyCompleted });
    await refreshHistory();
  }
</script>

<div class="view">
  <header class="head">
    <div>
      <h1>Historial</h1>
      <p class="subtitle">{counts.completed} completadas · {f.bytes(totalSize)} descargados{#if counts.failed} · {counts.failed} fallidas{/if}</p>
    </div>
    <div class="head-actions">
      <button class="btn" onclick={() => clear(true)} disabled={!counts.completed}><Icon name="check" size={15} /> Limpiar completados</button>
      <button class="btn ghost" onclick={() => clear(false)} disabled={!counts.all}><Icon name="trash" size={15} /> Vaciar</button>
    </div>
  </header>

  <div class="toolbar">
    <div class="tabs">
      {#each [['all', 'Todos'], ['completed', 'Completados'], ['failed', 'Fallidos']] as [k, label]}
        <button class="tab" class:sel={filter === k} onclick={() => (filter = k)}>
          {label} <span class="count">{counts[k]}</span>
          {#if filter === k}<span class="underline" in:fly={{ x: 0, duration: 0 }}></span>{/if}
        </button>
      {/each}
    </div>
    <div class="search">
      <Icon name="search" size={15} />
      <input class="input" placeholder="Buscar…" bind:value={search} />
    </div>
  </div>

  <div class="list">
    {#each items as h (h.id)}
      {@const ok = h.status === 'completed'}
      <article class="hist card" class:failed={!ok} animate:flip={{ duration: d(320), easing: cubicOut }} in:fly={{ y: 12, duration: d(280) }} out:fade={{ duration: d(160) }}>
        <div class="h-top" onclick={() => (open = open === h.id ? null : h.id)} role="presentation">
          <div class="h-icon {ok ? 'ok' : 'err'}"><Icon name={ok ? 'check' : 'x'} size={17} stroke={2.6} /></div>
          <div class="h-main">
            <div class="h-name truncate" title={h.name}>{h.name}</div>
            <div class="h-meta">
              <span>{f.bytes(h.size)}</span>
              <span>·</span>
              <span title={f.dateTime(h.completedAt)}>{f.ago(h.completedAt)}</span>
              {#if ok && h.avgSpeed}<span>·</span><span>media {f.speed(h.avgSpeed)}</span>{/if}
              {#if h.viaName}<span class="chip info" title="Descargado a través de {h.viaName}"><Icon name="cloud" size={10} stroke={2.6} /> {h.viaName}</span>{/if}
              {#if h.warning}<span class="chip warn"><Icon name="alert-triangle" size={10} stroke={2.6} /> Aviso</span>{/if}
              {#if !ok}<span class="chip err">Falló en {f.STAGES[h.failedStage] ?? h.failedStage}</span>{/if}
            </div>
          </div>
          <div class="h-actions" onclick={(e) => e.stopPropagation()} role="presentation">
            {#if ok}
              <button class="btn sm" onclick={() => invoke('open_path', { path: h.dest })}><Icon name="folder" size={14} /> Abrir</button>
            {:else}
              {#if h.canRetryExtract}
                <button class="btn sm" onclick={() => retryExtract(h)}><Icon name="key" size={14} /> Reintentar extracción</button>
              {/if}
              <button class="btn sm" onclick={() => retry(h)}><Icon name="refresh" size={14} /> Reintentar</button>
            {/if}
            <button class="icon-btn danger" onclick={() => remove(h)} title="Eliminar del historial"><Icon name="trash" size={15} /></button>
          </div>
        </div>
        {#if !ok && h.error}
          <div class="alert err"><Icon name="alert-triangle" size={15} /><span class="selectable">{h.error}</span></div>
        {/if}
        {#if open === h.id}
          <div class="h-details" transition:slide={{ duration: d(220) }}>
            <div><span class="k">Carpeta</span><span class="v selectable">{ok ? h.dest : h.workDir}</span></div>
            <div><span class="k">Añadido</span><span class="v">{f.dateTime(h.addedAt)}</span></div>
            <div><span class="k">Terminado</span><span class="v">{f.dateTime(h.completedAt)}</span></div>
            <div><span class="k">Duración</span><span class="v">{f.duration(h.downloadSecs)}</span></div>
            {#if h.warning}<div class="wide"><span class="k">Aviso</span><span class="v selectable">{h.warning}</span></div>{/if}
          </div>
        {/if}
      </article>
    {:else}
      <div class="empty" in:fade={{ duration: d(250) }}>
        <Icon name="history" size={36} stroke={1.6} />
        <p>{search ? 'Nada coincide con la búsqueda.' : 'Aquí aparecerán tus descargas terminadas.'}</p>
      </div>
    {/each}
  </div>
</div>

<style>
  .view { padding: 26px 30px 40px; max-width: 1100px; margin: 0 auto; }
  .head { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; }
  h1 { margin: 0; font-family: var(--font-display); font-size: 26px; font-weight: 700; }
  .subtitle { margin: 4px 0 0; color: var(--text-2); }
  .head-actions { display: flex; gap: 8px; }
  .toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-top: 20px; border-bottom: 1px solid var(--border); }
  .tabs { display: flex; gap: 4px; }
  .tab { position: relative; height: 40px; padding: 0 14px; border: none; background: none; color: var(--text-2); font-weight: 560; cursor: pointer; display: flex; align-items: center; gap: 8px; }
  .tab:hover { color: var(--text); }
  .tab.sel { color: var(--text); }
  .count { font-size: 11px; background: var(--panel-3); padding: 1px 7px; border-radius: 99px; color: var(--text-2); }
  .underline { position: absolute; left: 10px; right: 10px; bottom: -1px; height: 2px; border-radius: 2px; background: var(--grad); animation: grow 0.3s var(--ease); }
  @keyframes grow { from { transform: scaleX(0.3); opacity: 0; } }
  .search { position: relative; width: 240px; margin-bottom: 8px; }
  .search :global(svg) { position: absolute; left: 11px; top: 11px; color: var(--muted); }
  .search .input { height: 36px; padding-left: 34px; }
  .list { display: flex; flex-direction: column; gap: 10px; margin-top: 16px; }
  .hist { padding: 14px 16px; display: flex; flex-direction: column; gap: 12px; }
  .hist.failed { border-color: rgba(248, 113, 113, 0.22); }
  .h-top { display: flex; align-items: center; gap: 14px; cursor: pointer; }
  .h-icon { width: 36px; height: 36px; border-radius: 50%; display: grid; place-items: center; flex-shrink: 0; }
  .h-icon.ok { background: rgba(52, 211, 153, 0.14); color: var(--ok); }
  .h-icon.err { background: rgba(248, 113, 113, 0.14); color: var(--err); }
  .h-main { flex: 1; min-width: 0; }
  .h-name { font-weight: 600; }
  .h-meta { display: flex; align-items: center; gap: 7px; font-size: 12.5px; color: var(--text-2); margin-top: 3px; flex-wrap: wrap; }
  .h-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .h-details { display: grid; grid-template-columns: 2fr 1fr 1fr 1fr; gap: 10px 18px; padding-top: 12px; border-top: 1px solid var(--border); }
  .h-details > div { display: flex; flex-direction: column; min-width: 0; }
  .h-details .wide { grid-column: 1 / -1; }
  .k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.07em; color: var(--muted); font-weight: 650; }
  .v { font-size: 13px; margin-top: 2px; word-break: break-all; }
  .empty { text-align: center; padding: 60px 20px; color: var(--muted); display: flex; flex-direction: column; align-items: center; gap: 6px; }
</style>
