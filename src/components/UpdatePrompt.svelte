<script>
  // Aviso de versión nueva: novedades, descarga con su progreso e instalación. Se puede cerrar
  // (la actualización sigue en Configuración) u omitir esa versión.
  import { t, locale } from '../lib/i18n.svelte.js';
  import { scale } from 'svelte/transition';
  import Modal from './Modal.svelte';
  import Icon from './Icon.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import * as f from '../lib/format.js';
  import { ui, d } from '../lib/state.svelte.js';
  import { isAndroid } from '../lib/platform.js';
  import { upd, startUpdate, install, cancelDownload, closePrompt, skipVersion } from '../lib/updater.svelte.js';

  const info = $derived(upd.info);
  const pct = $derived(upd.total ? (upd.downloaded / upd.total) * 100 : 0);
  const busy = $derived(upd.phase === 'downloading' || upd.phase === 'installing');
  const activeJobs = $derived(ui.queue.jobs.filter((j) => j.status !== 'paused').length);
  const date = $derived(info?.publishedAt ? new Date(info.publishedAt).toLocaleDateString(locale(), { day: 'numeric', month: 'long', year: 'numeric' }) : '');

  /** Notas de la release en líneas sencillas: sin comentarios HTML ni la tabla de archivos. */
  const notes = $derived.by(() => {
    const raw = (info?.notes ?? '').replace(/<!--[\s\S]*?-->/g, '');
    const out = [];
    for (let line of raw.split(/\r?\n/)) {
      line = line.trim();
      if (!line || line.startsWith('|')) continue;
      line = line.replace(/\*\*(.+?)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1').replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
      if (/^#{1,6}\s/.test(line)) out.push({ kind: 'h', text: line.replace(/^#+\s*/, '') });
      else if (/^[-*•]\s/.test(line)) out.push({ kind: 'li', text: line.replace(/^[-*•]\s*/, '') });
      else out.push({ kind: 'p', text: line });
    }
    return out;
  });

  function onclose() {
    if (upd.phase !== 'installing') closePrompt();
  }
</script>

{#if upd.open && info?.available}
  <Modal width={520} {onclose} dismissable={upd.phase !== 'installing'}>
    <div class="hero">
      <div class="glow"></div>
      <div class="badge" class:turning={upd.phase === 'installing'}>
        <Icon name={upd.phase === 'installing' ? 'loader' : 'rocket'} size={26} stroke={2.2} />
      </div>
      <div class="h-text">
        <h2>{t('update.title')}</h2>
        <div class="versions">
          <span class="chip muted">v{info.current}</span>
          <Icon name="chevron-right" size={14} />
          <span class="chip accent">v{info.version}</span>
        </div>
        {#if date || info.assetSize}
          <p class="meta">{[date, info.assetSize ? f.bytes(info.assetSize) : ''].filter(Boolean).join(' · ')}</p>
        {/if}
      </div>
      {#if upd.phase !== 'installing'}
        <button class="icon-btn close" onclick={onclose} aria-label={t('common.close')}><Icon name="x" size={18} /></button>
      {/if}
    </div>

    {#if upd.phase === 'downloading' || upd.phase === 'ready' || upd.phase === 'permission'}
      <div class="progress" in:scale={{ start: 0.97, duration: d(200) }}>
        <div class="p-row">
          <strong>{upd.phase === 'downloading' ? t('update.downloading') : t('update.downloadedReady')}</strong>
          <span>{Math.round(upd.phase === 'downloading' ? pct : 100)}%</span>
        </div>
        <ProgressBar value={upd.phase === 'downloading' ? pct : 100} active={upd.phase === 'downloading'} kind={upd.phase === 'downloading' ? 'accent' : 'ok'} />
        {#if upd.phase === 'downloading'}
          <div class="p-sub">
            {f.bytes(upd.downloaded)} / {f.bytes(upd.total)}{#if upd.speed > 0}{' · '}{f.speed(upd.speed)}{/if}
          </div>
        {/if}
      </div>
      {#if upd.phase === 'permission'}
        <div class="alert warn"><Icon name="shield" size={16} /><div>{t('update.permission')}</div></div>
      {:else if upd.phase === 'ready' && isAndroid}
        <p class="hint">{t('update.androidHint')}</p>
      {/if}
    {:else if upd.phase === 'installing'}
      <p class="installing">{isAndroid ? t('update.openingInstaller') : t('update.restarting')}</p>
    {:else if notes.length}
      <div class="notes">
        <div class="n-title">{t('update.whatsNew')}</div>
        <div class="n-body">
          {#each notes as n}
            {#if n.kind === 'h'}<h4>{n.text}</h4>
            {:else if n.kind === 'li'}<div class="li">{n.text}</div>
            {:else}<p>{n.text}</p>{/if}
          {/each}
        </div>
      </div>
    {/if}

    {#if upd.phase === 'error'}
      <div class="alert err"><Icon name="alert-triangle" size={16} /><div>{upd.error}</div></div>
    {/if}
    {#if !isAndroid && activeJobs && (upd.phase === 'idle' || upd.phase === 'error' || upd.phase === 'downloading')}
      <p class="hint"><Icon name="info" size={13} /> {t('update.activeDownloads')}</p>
    {/if}
    {#if !info.kind}
      <p class="hint"><Icon name="info" size={13} /> {t('update.manualOnly')}</p>
    {/if}

    {#snippet footer()}
      {#if upd.phase === 'downloading'}
        <button class="btn ghost" onclick={cancelDownload}>{t('common.cancel')}</button>
        <button class="btn" onclick={closePrompt}>{t('update.background')}</button>
      {:else if upd.phase === 'installing'}
        <span class="muted-note">{t('update.dontClose')}</span>
      {:else if upd.phase === 'ready' || upd.phase === 'permission'}
        <button class="btn ghost" onclick={closePrompt}>{t('update.later')}</button>
        <button class="btn primary" onclick={install}><Icon name="package" size={15} /> {t('update.install')}</button>
      {:else}
        <button class="btn ghost skip" onclick={skipVersion}>{t('update.skip')}</button>
        <span class="spacer"></span>
        <button class="btn ghost" onclick={closePrompt}>{t('update.later')}</button>
        <button class="btn primary" onclick={startUpdate} disabled={busy}>
          <Icon name={info.kind ? 'download' : 'external'} size={15} />
          {upd.phase === 'error' ? t('common.retry') : info.kind ? t('update.now') : t('update.openPage')}
        </button>
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .hero {
    position: relative;
    display: flex; align-items: center; gap: 16px;
    margin: -16px -22px 0; padding: 26px 22px 20px;
    overflow: hidden;
    background:
      radial-gradient(120% 140% at 0% 0%, rgba(var(--accent-rgb), 0.28), transparent 60%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent);
    border-bottom: 1px solid var(--border);
  }
  .glow {
    position: absolute; right: -60px; top: -80px; width: 220px; height: 220px; border-radius: 50%;
    background: radial-gradient(circle, rgba(var(--accent-rgb), 0.35), transparent 70%);
    filter: blur(10px); pointer-events: none;
    animation: float 6s ease-in-out infinite alternate;
  }
  @keyframes float { to { transform: translate(-30px, 20px); } }
  .badge {
    position: relative; flex-shrink: 0;
    width: 58px; height: 58px; border-radius: 17px;
    display: grid; place-items: center; color: #fff;
    background: var(--grad-diag);
    box-shadow: 0 12px 32px -10px rgba(var(--accent-rgb), 0.95), inset 0 1px 0 rgba(255, 255, 255, 0.25);
  }
  .badge.turning :global(svg) { animation: spin 1s linear infinite; }
  .h-text { position: relative; flex: 1; min-width: 0; }
  h2 { margin: 0; font-family: var(--font-display); font-size: 19px; font-weight: 700; }
  .versions { display: flex; align-items: center; gap: 6px; margin-top: 8px; color: var(--muted); }
  .versions .chip { font-family: var(--mono); font-size: 12px; }
  .meta { margin: 8px 0 0; font-size: 12.5px; color: var(--muted); }
  .close { position: absolute; top: 12px; right: 12px; }
  :global([dir='rtl']) .close { right: auto; left: 12px; }

  .notes { margin-top: 16px; }
  .n-title { font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); margin-bottom: 8px; }
  .n-body {
    max-height: 220px; overflow-y: auto;
    padding: 12px 14px; border-radius: 12px;
    background: var(--panel-2); border: 1px solid var(--border);
    font-size: 13px; line-height: 1.55; color: var(--text-2);
  }
  .n-body h4 { margin: 10px 0 4px; font-size: 13.5px; color: var(--text); }
  .n-body h4:first-child { margin-top: 0; }
  .n-body p { margin: 0 0 6px; }
  .li { position: relative; padding-inline-start: 14px; margin-bottom: 4px; }
  .li::before { content: ''; position: absolute; inset-inline-start: 2px; top: 0.6em; width: 5px; height: 5px; border-radius: 50%; background: var(--accent-2); }

  .progress { margin-top: 18px; display: flex; flex-direction: column; gap: 8px; }
  .p-row { display: flex; justify-content: space-between; font-size: 13.5px; }
  .p-row span { font-variant-numeric: tabular-nums; color: var(--text-2); }
  .p-sub { font-size: 12.5px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .installing { margin: 18px 0 2px; color: var(--text-2); }
  .hint { display: flex; align-items: center; gap: 6px; margin: 14px 0 0; font-size: 12.5px; color: var(--muted); line-height: 1.5; }
  .alert { margin-top: 14px; display: flex; gap: 10px; align-items: flex-start; }
  .alert > div { flex: 1; min-width: 0; word-break: break-word; }
  .spacer { flex: 1; }
  .muted-note { font-size: 12.5px; color: var(--muted); }
  :global(html.mobile) .hero { margin: -14px -16px 0; padding: 22px 16px 18px; }
  :global(html.mobile) .skip { order: 3; width: 100%; }
  :global(html.mobile) .spacer { display: none; }
</style>
