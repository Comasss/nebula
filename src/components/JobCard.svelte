<script>
  import { invoke } from '@tauri-apps/api/core';
  import { slide } from 'svelte/transition';
  import Icon from './Icon.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { ui, ask, toast, d } from '../lib/state.svelte.js';
  import * as f from '../lib/format.js';

  let { job, index, count } = $props();
  let expanded = $state(false);
  let files = $state([]);

  const st = $derived(f.STATUS[job.status] ?? f.STATUS.queued);
  const pp = $derived(['verifying', 'repairing', 'extracting', 'moving'].includes(job.status));
  const percent = $derived(pp && job.post ? job.post.percent : job.percent);
  const rate = $derived(pp ? job.post?.speed ?? 0 : job.speed);
  const barKind = $derived(
    { downloading: 'accent', queued: 'muted', paused: 'paused', verifying: 'info', repairing: 'warn', extracting: 'teal', moving: 'teal' }[job.status] ?? 'accent'
  );
  const waiting = $derived(pp && job.post?.stage === 'En espera');

  $effect(() => {
    if (!expanded) return;
    let alive = true;
    const load = async () => {
      if (!alive) return;
      try {
        files = await invoke('get_job_files', { id: job.id });
      } catch {}
    };
    load();
    const t = setInterval(load, 1500);
    return () => {
      alive = false;
      clearInterval(t);
    };
  });

  function togglePause() {
    invoke(job.status === 'paused' ? 'resume_job' : 'pause_job', { id: job.id });
  }

  async function remove() {
    let deleteFiles = true;
    if (ui.settings?.confirmDelete) {
      const r = await ask({
        title: '¿Eliminar de la cola?',
        message: job.name,
        confirmLabel: 'Eliminar',
        danger: true,
        icon: 'trash',
        checkbox: 'Borrar también los archivos ya descargados',
        checkedDefault: true,
      });
      if (!r.ok) return;
      deleteFiles = r.checked;
    }
    await invoke('remove_job', { id: job.id, deleteFiles });
    toast('info', `Eliminado: ${job.name}`);
  }

  function move(to) {
    invoke('move_job', { id: job.id, to });
  }

  function subline() {
    if (pp) {
      return job.post?.detail ? `${job.post.stage} · ${job.post.detail}` : job.post?.stage ?? st.label;
    }
    const parts = [`${f.bytes(job.done + job.failed)} de ${f.bytes(job.total)}`];
    parts.push(`${job.filesDone}/${job.files} archivos`);
    return parts.join(' · ');
  }
</script>

<article class="job card {job.status}" class:expanded>
  <div class="top">
    <div class="s-icon {st.kind}" class:spinning={pp && !waiting}>
      {#if pp && !waiting}
        <svg class="ring" viewBox="0 0 44 44"><circle cx="22" cy="22" r="20" /></svg>
      {/if}
      <Icon name={st.icon} size={18} />
    </div>

    <div class="main">
      <div class="name truncate" title={job.name}>{job.name}</div>
      <div class="meta">
        <span class="chip {st.kind}">{waiting ? 'En espera' : st.label}</span>
        <span class="sub truncate">{subline()}</span>
        {#if job.heldPar2 > 0 && ['queued', 'downloading', 'paused'].includes(job.status)}
          <span class="chip muted" title="Los volúmenes de recuperación PAR2 ({job.heldPar2}) solo se descargarán si la verificación encuentra archivos dañados.">
            <Icon name="shield" size={11} stroke={2.4} /> PAR2 en reserva
          </span>
        {/if}
        {#if job.failedArticles > 0}
          <span class="chip warn" title="Artículos que no se encontraron en ningún servidor. Se intentarán reparar con PAR2.">
            <Icon name="alert-triangle" size={11} stroke={2.5} />
            {job.failedArticles} faltan
          </span>
        {/if}
      </div>
    </div>

    <div class="stats">
      {#if rate > 0}
        <div class="rate">{f.speed(rate)}</div>
      {/if}
      {#if job.status === 'downloading' && job.eta > 0}
        <div class="eta"><Icon name="clock" size={12} /> {f.duration(job.eta)}</div>
      {:else if job.status === 'queued'}
        <div class="eta">#{index + 1} en cola</div>
      {/if}
    </div>

    <div class="actions">
      {#if !pp}
        <button class="icon-btn" onclick={() => move(0)} disabled={index === 0} title="Mover al principio"><Icon name="arrow-up-to-line" size={16} /></button>
        <button class="icon-btn" onclick={() => move(index - 1)} disabled={index === 0} title="Subir"><Icon name="chevron-up" size={17} /></button>
        <button class="icon-btn" onclick={() => move(index + 1)} disabled={index >= count - 1} title="Bajar"><Icon name="chevron-down" size={17} /></button>
        <button class="icon-btn" onclick={togglePause} title={job.status === 'paused' ? 'Reanudar' : 'Pausar'}>
          <Icon name={job.status === 'paused' ? 'play' : 'pause'} size={16} />
        </button>
      {/if}
      <button class="icon-btn" onclick={() => invoke('open_path', { path: job.dest })} title="Abrir carpeta de destino"><Icon name="folder" size={16} /></button>
      <button class="icon-btn danger" onclick={remove} title="Eliminar"><Icon name="trash" size={16} /></button>
      <button class="icon-btn chevron" class:open={expanded} onclick={() => (expanded = !expanded)} title="Detalles"><Icon name="chevron-down" size={17} /></button>
    </div>
  </div>

  <div class="bar-row">
    <ProgressBar value={percent} kind={barKind} active={(job.status === 'downloading' && !ui.queue.paused) || (pp && !waiting)} />
    <span class="pct">{percent.toFixed(1)}%</span>
  </div>

  {#if job.error}
    <div class="alert err" transition:slide={{ duration: d(200) }}><Icon name="x-circle" size={16} /><span class="selectable">{job.error}</span></div>
  {/if}
  {#if job.warning}
    <div class="alert warn" transition:slide={{ duration: d(200) }}><Icon name="alert-triangle" size={16} /><span class="selectable">{job.warning}</span></div>
  {/if}
  {#if job.status === 'downloading' && ui.queue.noServers}
    <div class="alert warn" transition:slide={{ duration: d(200) }}>
      <Icon name="wifi" size={16} /><span>Esperando conexión con algún servidor… Revisa la pestaña Servidores.</span>
    </div>
  {/if}

  {#if expanded}
    <div class="details" transition:slide={{ duration: d(260) }}>
      <div class="d-grid">
        <div><span class="k">Destino</span><span class="v truncate selectable" title={job.dest}>{job.dest}</span></div>
        <div><span class="k">Tamaño</span><span class="v">{f.bytes(job.total)}</span></div>
        <div><span class="k">Añadido</span><span class="v">{f.ago(job.addedAt)}</span></div>
        {#if job.heldPar2 > 0}
          <div><span class="k">PAR2 en reserva</span><span class="v">{job.heldPar2} volúmenes (solo si hacen falta)</span></div>
        {/if}
      </div>
      {#if files.length}
        <div class="files">
          {#each files as file (file.name)}
            {@const fp = file.bytes ? ((file.done + file.failed) / file.bytes) * 100 : 0}
            <div class="file" class:held={file.held}>
              <Icon name={file.par2 ? 'shield' : 'file'} size={14} />
              <span class="fname truncate" title={file.name}>{file.name}</span>
              <span>
                {#if file.held}
                  <span class="chip muted">reserva</span>
                {:else if file.failedSegs > 0}
                  <span class="chip warn">{file.failedSegs} faltan</span>
                {/if}
              </span>
              <span class="fsize">{f.bytes(file.bytes)}</span>
              <div class="fbar"><ProgressBar value={file.held ? 0 : fp} kind={file.complete ? (file.failedSegs ? 'warn' : 'ok') : 'accent'} height={4} /></div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</article>

<style>
  .job {
    padding: 16px 16px 14px 16px;
    transition: border-color 0.25s, box-shadow 0.3s, transform 0.2s var(--ease);
  }
  .job:hover { border-color: var(--border-2); }
  .job.downloading { border-color: rgba(var(--accent-rgb), 0.28); box-shadow: 0 10px 30px -18px rgba(var(--accent-rgb), 0.8); }
  .top { display: flex; align-items: center; gap: 14px; }
  .s-icon {
    position: relative;
    width: 42px; height: 42px;
    border-radius: 12px;
    display: grid; place-items: center;
    flex-shrink: 0;
    background: var(--panel-3);
    color: var(--text-2);
  }
  .s-icon.accent { background: rgba(var(--accent-rgb), 0.16); color: var(--accent-2); }
  .s-icon.info { background: rgba(96, 165, 250, 0.14); color: var(--info); }
  .s-icon.warn { background: rgba(251, 191, 36, 0.14); color: var(--warn); }
  .s-icon.teal { background: rgba(45, 212, 191, 0.14); color: #2dd4bf; }
  .s-icon.err { background: rgba(248, 113, 113, 0.14); color: var(--err); }
  .downloading .s-icon :global(svg) { animation: drip 1.6s ease-in-out infinite; }
  @keyframes drip { 50% { transform: translateY(2px); } }
  .ring { position: absolute; inset: -1px; width: 44px; height: 44px; animation: spin 1.4s linear infinite; }
  .ring circle { fill: none; stroke: currentColor; stroke-width: 2; stroke-dasharray: 30 100; stroke-linecap: round; opacity: 0.8; }
  .main { flex: 1; min-width: 0; }
  .name { font-weight: 620; font-size: 14.5px; }
  .meta { display: flex; align-items: center; gap: 8px; margin-top: 5px; min-width: 0; }
  .sub { font-size: 12.5px; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .stats { text-align: right; flex-shrink: 0; min-width: 90px; }
  .rate { font-family: var(--font-display); font-weight: 650; font-size: 15px; font-variant-numeric: tabular-nums; }
  .eta { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--muted); margin-top: 2px; font-variant-numeric: tabular-nums; }
  .actions { display: flex; gap: 2px; flex-shrink: 0; }
  .actions .icon-btn { opacity: 0.75; }
  .job:hover .actions .icon-btn { opacity: 1; }
  .chevron :global(svg) { transition: transform 0.3s var(--ease); }
  .chevron.open :global(svg) { transform: rotate(180deg); }
  .bar-row { display: flex; align-items: center; gap: 12px; margin-top: 13px; }
  .pct { width: 52px; text-align: right; font-size: 12.5px; font-weight: 650; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .alert { margin-top: 12px; }
  .details { margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--border); }
  .d-grid { display: grid; grid-template-columns: 2fr 1fr 1fr; gap: 10px 18px; margin-bottom: 12px; }
  .d-grid > div { display: flex; flex-direction: column; min-width: 0; }
  .k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.07em; color: var(--muted); font-weight: 650; }
  .v { font-size: 13px; margin-top: 2px; }
  .files { max-height: 260px; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; margin: 0 -6px; padding: 0 6px; }
  .file { display: grid; grid-template-columns: 16px 1fr auto 76px 120px; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 8px; font-size: 12.5px; color: var(--text-2); }
  .file:hover { background: var(--panel-2); }
  .file.held { opacity: 0.55; }
  .fname { color: var(--text); }
  .fsize { text-align: right; font-variant-numeric: tabular-nums; }
</style>
