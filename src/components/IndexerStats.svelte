<script>
  import { t, num } from '../lib/i18n.svelte.js';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { ui, d } from '../lib/state.svelte.js';

  // events: peticiones a indexadores (búsquedas y descargas de NZB); records: NZB terminados.
  let { events = [], records = [], period, loaded = false } = $props();

  const within = (at) => period.all || (at >= period.from && at <= period.to);
  const inRange = $derived(events.filter((e) => within(e.at)));
  const requests = $derived(inRange.filter((e) => e.kind === 'search'));
  const firstPages = $derived(requests.filter((e) => !e.offset));
  const grabs = $derived(inRange.filter((e) => e.kind === 'grab'));
  const downloads = $derived(records.filter((r) => r.indexer && within(r.at)));

  const avg = (list, f) => (list.length ? list.reduce((a, x) => a + f(x), 0) / list.length : null);

  // Una búsqueda agrupa la primera página de cada indexador consultado a la vez.
  const searches = $derived.by(() => {
    const map = new Map();
    firstPages.forEach((e, i) => {
      const k = e.search || `#${i}`;
      const s = map.get(k) ?? { results: 0, ok: false };
      if (e.ok) {
        s.results += e.results;
        s.ok = true;
      }
      map.set(k, s);
    });
    return [...map.values()];
  });

  const avgMs = $derived(avg(requests, (e) => e.ms));
  const failed = $derived(requests.filter((e) => !e.ok).length);
  const okSearches = $derived(searches.filter((s) => s.ok));
  const avgResults = $derived(avg(okSearches, (s) => s.results));
  const totalResults = $derived(okSearches.reduce((a, s) => a + s.results, 0));
  const grabsOk = $derived(grabs.filter((e) => e.ok).length);
  const dlOk = $derived(downloads.filter((r) => r.ok).length);

  function label(id, fallback) {
    const ix = ui.settings?.indexers?.find((x) => x.id === id);
    return ix ? { name: ix.name || ix.url, gone: false } : { name: fallback || id, gone: true };
  }

  const rows = $derived.by(() => {
    const map = new Map();
    const row = (id, name) => {
      let r = map.get(id);
      if (!r) map.set(id, (r = { id, stored: name, req: 0, msSum: 0, min: Infinity, max: 0, failed: 0, pages: 0, results: 0, grabOk: 0, grabFail: 0, dl: 0, dlOk: 0 }));
      if (name) r.stored = name;
      return r;
    };
    for (const e of requests) {
      const r = row(e.id, e.name);
      r.req++;
      r.msSum += e.ms;
      r.min = Math.min(r.min, e.ms);
      r.max = Math.max(r.max, e.ms);
      if (!e.ok) r.failed++;
      if (!e.offset && e.ok) {
        r.pages++;
        r.results += e.results;
      }
    }
    for (const e of grabs) {
      const r = row(e.id, e.name);
      if (e.ok) r.grabOk++;
      else r.grabFail++;
    }
    for (const rec of downloads) {
      const r = row(rec.indexer, '');
      r.dl++;
      if (rec.ok) r.dlOk++;
    }
    const order = (id) => {
      const i = ui.settings?.indexers?.findIndex((x) => x.id === id) ?? -1;
      return i < 0 ? 999 : i;
    };
    return [...map.values()]
      .map((r) => ({
        ...r,
        ...label(r.id, r.stored),
        searches: requests.filter((e) => e.id === r.id && !e.offset).length,
        avgMs: r.req ? r.msSum / r.req : null,
        errRate: r.req ? (r.failed / r.req) * 100 : null,
        avgResults: r.pages ? r.results / r.pages : null,
        dlRate: r.dl ? (r.dlOk / r.dl) * 100 : null,
      }))
      .sort((a, b) => order(a.id) - order(b.id));
  });

  const maxMs = $derived(Math.max(1, ...rows.map((r) => r.avgMs ?? 0)));
  const fastest = $derived(rows.filter((r) => r.avgMs !== null && r.req >= 1).sort((a, b) => a.avgMs - b.avgMs)[0]);

  const ms = (v) => (v === null ? '—' : `${num(Math.round(v))} ms`);
  const pct = (v) => (v === null ? '—' : `${num(v, { maximumFractionDigits: 1, minimumFractionDigits: v > 0 && v < 100 ? 1 : 0 })}\u00a0%`);
  const one = (v) => (v === null ? '—' : num(v, { maximumFractionDigits: 1 }));
  const msTone = (v) => (v === null ? 'muted' : v < 1000 ? 'ok' : v < 2500 ? 'warn' : 'err');
  const errTone = (v) => (v === null || v < 5 ? 'muted' : v < 15 ? 'warn' : 'err');
  const rateTone = (v) => (v === null ? 'muted' : v >= 90 ? 'ok' : v >= 60 ? 'warn' : 'err');
</script>

<div class="kpis">
  <div class="kpi card">
    <div class="k-label"><Icon name="search" size={14} /> {t('stats.ix.searches')}</div>
    <div class="k-value">{num(searches.length)}</div>
    <div class="k-foot">{t('stats.ix.requests', { n: num(requests.length), failed: num(failed) })}</div>
  </div>
  <div class="kpi card main">
    <div class="k-label"><Icon name="clock" size={14} /> {t('stats.ix.avgTime')}</div>
    <div class="k-value grad-text">{ms(avgMs)}</div>
    <div class="k-foot truncate">{fastest && rows.length > 1 ? t('stats.ix.fastest', { name: fastest.name }) : ' '}</div>
  </div>
  <div class="kpi card">
    <div class="k-label"><Icon name="layers" size={14} /> {t('stats.ix.avgResults')}</div>
    <div class="k-value">{one(avgResults)}</div>
    <div class="k-foot">{t('stats.ix.totalResults', { n: num(totalResults) })}</div>
  </div>
  <div class="kpi card">
    <div class="k-label"><Icon name="download" size={14} /> {t('stats.ix.grabs')}</div>
    <div class="k-value">{num(grabsOk)}</div>
    <div class="k-foot">{downloads.length ? t('stats.ix.grabsFoot', { rate: pct((dlOk / downloads.length) * 100) }) : ' '}</div>
  </div>
</div>

<section>
  <div class="sec-head">
    <h2><Icon name="layers" size={17} /> {t('nav.indexers')}</h2>
    <p class="hint">{t('stats.ix.hint')}</p>
  </div>
  {#if rows.length}
    <div class="table card">
      <div class="tr th">
        <span>{t('stats.ix.col.indexer')}</span>
        <span class="r">{t('stats.ix.searches')}</span>
        <span>{t('stats.ix.col.avgTime')}</span>
        <span class="r">{t('stats.ix.col.avgResults')}</span>
        <span class="r">{t('stats.ix.col.errors')}</span>
        <span class="r">{t('stats.ix.grabs')}</span>
        <span class="r">{t('stats.ix.col.downloads')}</span>
      </div>
      {#each rows as r (r.id)}
        <div class="tr" in:fly={{ y: 8, duration: d(260), easing: cubicOut }}>
          <span class="name">
            <span class="truncate" title={r.name}>{r.name}</span>
            {#if r.gone}<span class="chip muted">{t('stats.removed')}</span>{/if}
          </span>
          <span class="r num">{num(r.searches)}</span>
          <span class="rate" title={r.req ? t('stats.ix.timeTip', { min: num(r.min), max: num(r.max) }) : ''}>
            <span class="bar"><span class={msTone(r.avgMs)} style="width:{((r.avgMs ?? 0) / maxMs) * 100}%"></span></span>
            <span class="num val">{ms(r.avgMs)}</span>
          </span>
          <span class="r num">{one(r.avgResults)}</span>
          <span class="r num {errTone(r.errRate)}-t">{pct(r.errRate)}</span>
          <span class="r num" title={t('stats.ix.grabTip', { ok: num(r.grabOk), failed: num(r.grabFail) })}>
            {num(r.grabOk)}{#if r.grabFail}<span class="err-t small"> +{num(r.grabFail)}</span>{/if}
          </span>
          <span class="r num {rateTone(r.dlRate)}-t" title={r.dl ? t('stats.ix.dlTip', { ok: num(r.dlOk), total: num(r.dl) }) : ''}>{pct(r.dlRate)}</span>
        </div>
      {/each}
    </div>
  {:else if loaded}
    <div class="empty card" in:fade={{ duration: d(200) }}>
      <Icon name="layers" size={28} stroke={1.6} />
      <p>{t('stats.ix.noData')}</p>
    </div>
  {/if}
</section>

<style>
  .kpis { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-top: 18px; }
  .kpi { padding: 16px 18px; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .kpi.main { border-color: rgba(var(--accent-rgb), 0.3); background: linear-gradient(160deg, rgba(var(--accent-rgb), 0.12), transparent 70%), var(--panel); }
  .k-label { display: flex; align-items: center; gap: 7px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.07em; color: var(--muted); font-weight: 650; }
  .k-value { font-family: var(--font-display); font-size: 30px; font-weight: 700; font-variant-numeric: tabular-nums; line-height: 1.15; }
  .k-foot { font-size: 12.5px; color: var(--text-2); }

  .bar { position: relative; height: 6px; border-radius: 99px; background: var(--panel-3); overflow: hidden; flex: 1; }
  .bar > span { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 99px; transition: width 0.6s var(--ease); }
  .bar > .ok { background: linear-gradient(90deg, #10b981, var(--ok)); }
  .bar > .warn { background: linear-gradient(90deg, #f59e0b, var(--warn)); }
  .bar > .err { background: linear-gradient(90deg, #ef4444, var(--err)); }
  .bar > .muted { background: transparent; }

  section { margin-top: 28px; }
  .sec-head { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; margin-bottom: 10px; }
  h2 { margin: 0; font-family: var(--font-display); font-size: 17px; font-weight: 650; display: flex; align-items: center; gap: 8px; }
  h2 :global(svg) { color: var(--accent-2); }

  .table { overflow: hidden; }
  .tr {
    display: grid;
    grid-template-columns: minmax(140px, 1.6fr) 84px minmax(160px, 1.6fr) 84px 72px 100px 96px;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    font-size: 13.5px;
  }
  .tr:first-child { border-top: none; }
  .tr:not(.th):hover { background: rgba(255, 255, 255, 0.02); }
  .th { padding-top: 10px; padding-bottom: 10px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.07em; color: var(--muted); font-weight: 650; background: rgba(255, 255, 255, 0.015); }
  .r { text-align: right; }
  .num { font-variant-numeric: tabular-nums; }
  .name { display: flex; align-items: center; gap: 8px; min-width: 0; font-weight: 600; }
  .rate { display: flex; align-items: center; gap: 10px; }
  .val { width: 72px; text-align: right; font-weight: 650; }
  .small { font-size: 12px; }
  .ok-t { color: #6ee7b7; }
  .warn-t { color: #fcd34d; }
  .err-t { color: #fca5a5; }
  .muted-t { color: var(--muted); }

  .empty { text-align: center; padding: 34px 20px; color: var(--muted); display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .empty p { margin: 6px 0 0; max-width: 460px; }

  @media (max-width: 980px) {
    .kpis { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
