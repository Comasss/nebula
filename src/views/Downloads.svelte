<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { fly, fade, scale } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import Icon from '../components/Icon.svelte';
  import JobCard from '../components/JobCard.svelte';
  import SpeedGraph from '../components/SpeedGraph.svelte';
  import { ui, queueAdd, toast, d } from '../lib/state.svelte.js';
  import * as f from '../lib/format.js';
  import { usableDebrid } from '../lib/debrid.js';

  let url = $state('');
  let limitOpen = $state(false);
  let customLimit = $state('');

  const jobs = $derived(ui.queue.jobs);
  const activeJobs = $derived(jobs.filter((j) => j.status === 'downloading').length);
  const totalConns = $derived(ui.queue.servers.reduce((a, s) => a + s.maxConns, 0));
  const peak = $derived(Math.max(...ui.speedHistory));
  const noServerConfigured = $derived(!ui.settings?.servers?.length && !usableDebrid(ui.settings).length);
  const serverErrors = $derived(ui.queue.servers.filter((s) => ['auth', 'down', 'warn'].includes(s.state) && s.lastError));

  const LIMITS = [
    { kbps: 0, label: 'Sin límite' },
    { kbps: 1024, label: '1 MB/s' },
    { kbps: 5 * 1024, label: '5 MB/s' },
    { kbps: 10 * 1024, label: '10 MB/s' },
    { kbps: 25 * 1024, label: '25 MB/s' },
    { kbps: 50 * 1024, label: '50 MB/s' },
  ];

  function submitUrl() {
    const u = url.trim();
    if (!u) return;
    if (!/^https?:\/\/\S+$/i.test(u)) {
      toast('error', 'Introduce un enlace que empiece por http:// o https://');
      return;
    }
    queueAdd([{ kind: 'url', value: u }]);
    url = '';
  }

  function onPaste(e) {
    const text = (e.clipboardData?.getData('text') ?? '').trim();
    const lines = text.split(/\s+/).filter((l) => /^https?:\/\/\S+$/i.test(l));
    if (lines.length && lines.length === text.split(/\s+/).length) {
      e.preventDefault();
      queueAdd(lines.map((value) => ({ kind: 'url', value })));
      url = '';
    }
  }

  async function addFiles() {
    const sel = await open({
      multiple: true,
      title: 'Selecciona archivos NZB',
      filters: [{ name: 'Archivos NZB', extensions: ['nzb', 'gz'] }],
    });
    if (!sel) return;
    const arr = Array.isArray(sel) ? sel : [sel];
    queueAdd(arr.map((p) => ({ kind: 'file', value: p })));
  }

  async function setLimit(kbps) {
    await invoke('set_speed_limit', { kbps });
    if (ui.settings) ui.settings.speedLimitKbps = kbps;
    limitOpen = false;
    toast('info', kbps ? `Velocidad limitada a ${f.speed(kbps * 1024)}` : 'Límite de velocidad desactivado', 2500);
  }

  function applyCustom() {
    const mb = parseFloat(customLimit.replace(',', '.'));
    if (!isNaN(mb) && mb >= 0) setLimit(Math.round(mb * 1024));
    customLimit = '';
  }
</script>

<svelte:window onclick={() => (limitOpen = false)} />

<div class="view">
  <header class="head">
    <div>
      <h1>Descargas</h1>
      <p class="subtitle">
        {#if jobs.length === 0}
          No hay nada en la cola
        {:else}
          {jobs.length} en cola{#if ui.queue.remaining > 0} · {f.bytes(ui.queue.remaining)} restantes{/if}{#if ui.queue.eta > 0} · termina en {f.duration(ui.queue.eta)}{/if}
        {/if}
      </p>
    </div>
    <div class="head-actions">
      <button class="btn" onclick={() => invoke('set_paused', { paused: !ui.queue.paused })} disabled={!jobs.length && !ui.queue.paused}>
        <Icon name={ui.queue.paused ? 'play' : 'pause'} size={16} />
        {ui.queue.paused ? 'Reanudar todo' : 'Pausar todo'}
      </button>
      <button class="btn primary" onclick={addFiles}>
        <Icon name="file-plus" size={17} /> Añadir NZB
      </button>
    </div>
  </header>

  <form class="url-bar" onsubmit={(e) => { e.preventDefault(); submitUrl(); }}>
    <Icon name="link" size={18} />
    <input
      class="url-input"
      placeholder="Pega aquí el enlace directo a un .nzb y te preguntaré dónde guardarlo…"
      bind:value={url}
      onpaste={onPaste}
      spellcheck="false" />
    {#if url.trim()}
      <button class="btn primary sm" type="submit" transition:scale={{ start: 0.8, duration: d(180) }}>
        <Icon name="search" size={14} /> Detectar
      </button>
    {:else}
      <span class="kbd">Ctrl+V</span>
    {/if}
  </form>

  {#if noServerConfigured}
    <div class="alert warn banner" transition:fly={{ y: -8, duration: d(250) }}>
      <Icon name="server" size={18} />
      <div class="spacer">No has configurado ningún servidor Usenet ni servicio debrid. Las descargas no empezarán hasta que añadas uno.</div>
      <button class="btn sm" onclick={() => (ui.view = 'debrid')}><Icon name="cloud" size={14} /> Debrid</button>
      <button class="btn sm" onclick={() => (ui.setupOpen = true)}>Configurar servidor</button>
    </div>
  {:else if serverErrors.length && jobs.length}
    <div class="alert err banner" transition:fly={{ y: -8, duration: d(250) }}>
      <Icon name="plug" size={18} />
      <div class="spacer">
        {#each serverErrors as s}
          <div><strong>{s.name}:</strong> {s.lastError ?? 'sin conexión'}</div>
        {/each}
      </div>
      <button class="btn sm" onclick={() => (ui.view = 'servers')}>Ver servidores</button>
    </div>
  {/if}

  <section class="stats">
    <div class="stat card speed">
      <div class="st-top">
        <div>
          <div class="st-label">Velocidad actual</div>
          <div class="st-value big">{ui.queue.paused ? 'En pausa' : f.speed(ui.queue.speed)}</div>
        </div>
        <div class="limit-wrap">
          <button
            class="limit-btn"
            class:on={ui.queue.speedLimit > 0}
            onclick={(e) => { e.stopPropagation(); limitOpen = !limitOpen; }}
            title="Límite de velocidad">
            <Icon name="gauge" size={14} />
            {ui.queue.speedLimit > 0 ? `Máx. ${f.speed(ui.queue.speedLimit * 1024)}` : 'Sin límite'}
          </button>
          {#if limitOpen}
            <div class="popover card" transition:scale={{ start: 0.95, duration: d(160), easing: cubicOut }} onclick={(e) => e.stopPropagation()} role="presentation">
              {#each LIMITS as l}
                <button class="pop-item" class:sel={ui.queue.speedLimit === l.kbps} onclick={() => setLimit(l.kbps)}>
                  {l.label}
                  {#if ui.queue.speedLimit === l.kbps}<Icon name="check" size={14} />{/if}
                </button>
              {/each}
              <form class="custom" onsubmit={(e) => { e.preventDefault(); applyCustom(); }}>
                <input class="input" placeholder="Otro (MB/s)" bind:value={customLimit} />
                <button class="btn sm" type="submit">OK</button>
              </form>
            </div>
          {/if}
        </div>
      </div>
      <div class="graph"><SpeedGraph data={ui.speedHistory} height={64} id="main" /></div>
      <div class="st-foot">Pico en el último minuto: {f.speed(peak)}</div>
    </div>
    <div class="stat card">
      <div class="st-icon"><Icon name="hard-drive" size={18} /></div>
      <div class="st-label">Pendiente</div>
      <div class="st-value">{f.bytes(ui.queue.remaining)}</div>
      <div class="st-foot">{activeJobs ? `${activeJobs} descargando` : 'Nada descargando'}</div>
    </div>
    <div class="stat card">
      <div class="st-icon"><Icon name="clock" size={18} /></div>
      <div class="st-label">Tiempo estimado</div>
      <div class="st-value">{f.duration(ui.queue.eta)}</div>
      <div class="st-foot">{ui.queue.eta > 0 ? `A las ${new Date(Date.now() + ui.queue.eta * 1000).toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })}` : '—'}</div>
    </div>
    <div class="stat card">
      <div class="st-icon"><Icon name="plug" size={18} /></div>
      <div class="st-label">Conexiones</div>
      <div class="st-value">{ui.queue.activeConns}<span class="of">/{totalConns}</span></div>
      <div class="st-foot">{ui.queue.servers.length} servidor{ui.queue.servers.length === 1 ? '' : 'es'} activo{ui.queue.servers.length === 1 ? '' : 's'}</div>
    </div>
  </section>

  <section class="list">
    {#each jobs as job, i (job.id)}
      <div class="item" animate:flip={{ duration: d(380), easing: cubicOut }} in:fly={{ y: 18, duration: d(360), easing: cubicOut }} out:fade={{ duration: d(200) }}>
        <JobCard {job} index={i} count={jobs.length} />
      </div>
    {:else}
      <div class="empty" in:fade={{ duration: d(300), delay: d(150) }}>
        <div class="orb">
          <div class="orb-ring"></div>
          <Icon name="download" size={34} stroke={1.8} />
        </div>
        <h3>Todo tranquilo por aquí</h3>
        <p>Arrastra un archivo <strong>.nzb</strong> a la ventana, pega un enlace arriba o copia uno al portapapeles.</p>
        <div class="empty-actions">
          <button class="btn primary" onclick={addFiles}><Icon name="file-plus" size={16} /> Elegir archivo NZB</button>
          <button class="btn" onclick={() => (ui.view = 'history')}><Icon name="history" size={16} /> Ver historial</button>
        </div>
      </div>
    {/each}
  </section>
</div>

<style>
  .view { padding: 26px 30px 40px; max-width: 1240px; margin: 0 auto; }
  .head { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; }
  h1 { margin: 0; font-family: var(--font-display); font-size: 26px; font-weight: 700; letter-spacing: -0.01em; }
  .subtitle { margin: 4px 0 0; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .head-actions { display: flex; gap: 10px; }

  .url-bar {
    margin-top: 20px;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 52px;
    padding: 0 8px 0 18px;
    border-radius: 14px;
    border: 1px solid var(--border-2);
    background: var(--panel);
    color: var(--muted);
    transition: border-color 0.2s, box-shadow 0.25s;
  }
  .url-bar:focus-within {
    border-color: rgba(var(--accent-rgb), 0.7);
    box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.12), 0 10px 30px -15px rgba(var(--accent-rgb), 0.7);
    color: var(--accent-2);
  }
  .url-input { flex: 1; height: 100%; border: none; outline: none; background: transparent; color: var(--text); font-size: 14px; user-select: text; }
  .url-input::placeholder { color: var(--muted); }
  .kbd { font-size: 11px; font-weight: 650; color: var(--muted); border: 1px solid var(--border-2); border-radius: 6px; padding: 3px 7px; margin-right: 8px; }
  .banner { margin-top: 14px; align-items: center; }

  .stats { display: grid; grid-template-columns: 2.2fr 1fr 1fr 1fr; gap: 14px; margin-top: 18px; }
  .stat { padding: 16px 18px; position: relative; overflow: visible; display: flex; flex-direction: column; }
  .stat.speed { padding-bottom: 12px; z-index: 5; }
  .st-top { display: flex; justify-content: space-between; align-items: flex-start; }
  .st-icon {
    width: 32px; height: 32px; border-radius: 9px;
    display: grid; place-items: center;
    background: var(--panel-3); color: var(--accent-2);
    margin-bottom: 12px;
  }
  .st-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 650; }
  .st-value { font-family: var(--font-display); font-size: 21px; font-weight: 650; margin-top: 3px; font-variant-numeric: tabular-nums; }
  .st-value.big { font-size: 26px; }
  .of { color: var(--muted); font-size: 15px; }
  .st-foot { font-size: 12px; color: var(--muted); margin-top: auto; padding-top: 6px; }
  .graph { margin: 6px -18px 0; overflow: hidden; }
  .limit-wrap { position: relative; }
  .limit-btn {
    display: inline-flex; align-items: center; gap: 6px;
    height: 28px; padding: 0 10px;
    border-radius: 8px;
    border: 1px solid var(--border-2);
    background: var(--panel-2);
    color: var(--text-2);
    font-size: 12px; font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }
  .limit-btn:hover { color: var(--text); background: var(--panel-3); }
  .limit-btn.on { border-color: rgba(251, 191, 36, 0.4); color: #fcd34d; background: rgba(251, 191, 36, 0.08); }
  .popover {
    position: absolute; right: 0; top: 34px; z-index: 30;
    width: 200px; padding: 6px;
    max-height: min(360px, calc(100vh - 220px));
    overflow-y: auto;
    background: var(--panel-2);
    box-shadow: var(--shadow);
    transform-origin: top right;
  }
  .pop-item {
    width: 100%; height: 32px;
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 10px; border: none; border-radius: 7px;
    background: transparent; cursor: pointer; font-size: 13px;
  }
  .pop-item:hover { background: var(--panel-3); }
  .pop-item.sel { color: var(--accent-2); font-weight: 600; }
  .custom { display: flex; gap: 6px; padding: 6px 2px 2px; border-top: 1px solid var(--border); margin-top: 4px; }
  .custom .input { height: 30px; font-size: 12.5px; }

  .list { display: flex; flex-direction: column; gap: 12px; margin-top: 22px; }

  .empty { text-align: center; padding: 56px 20px 40px; }
  .orb {
    position: relative;
    width: 96px; height: 96px;
    margin: 0 auto 22px;
    border-radius: 50%;
    display: grid; place-items: center;
    color: #fff;
    background: radial-gradient(circle at 30% 25%, rgba(255, 255, 255, 0.35), transparent 45%), var(--grad-diag);
    box-shadow: 0 20px 60px -15px rgba(var(--accent-rgb), 0.8);
    animation: float 4s ease-in-out infinite;
  }
  .orb-ring {
    position: absolute; inset: -14px -30px;
    border-radius: 50%;
    border: 2px solid rgba(var(--accent-rgb), 0.45);
    transform: rotate(-18deg);
    animation: ring 6s linear infinite;
  }
  @keyframes float { 50% { transform: translateY(-10px); } }
  @keyframes ring { to { transform: rotate(342deg); } }
  .empty h3 { margin: 0; font-family: var(--font-display); font-size: 19px; }
  .empty p { margin: 8px auto 0; color: var(--text-2); max-width: 420px; line-height: 1.5; }
  .empty-actions { display: flex; gap: 10px; justify-content: center; margin-top: 22px; }
</style>
