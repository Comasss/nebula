<script>
  import { t, tn, num } from '../lib/i18n.svelte.js';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { fly, fade, scale, slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from '../components/Icon.svelte';
  import { ui, toast, queueAdd, d } from '../lib/state.svelte.js';
  import { CATEGORIES, SORTS, groupResults, sortGroups } from '../lib/search.js';
  import * as f from '../lib/format.js';

  const STORE = 'nebula.search';
  function loadPrefs() {
    try {
      return JSON.parse(localStorage.getItem(STORE) ?? '{}');
    } catch {
      return {};
    }
  }
  const saved = loadPrefs();

  let query = $state('');
  let category = $state(saved.category ?? '');
  let sort = $state(saved.sort ?? 'date');
  let selected = $state(saved.selected ?? null); // null = todos
  let pickerOpen = $state(false);
  let expanded = $state(null);
  let searched = $state(false);
  let lastQuery = '';
  let generation = 0;

  // Por indexador: { state: 'loading'|'ok'|'error', items, total, offset, count, ms, error }
  let perIndexer = $state({});
  // Estado de acciones por enlace: 'working' | 'queued' | 'saving' | 'saved' | 'error'
  let actions = $state({});

  const nntp = $derived(ui.settings?.featureNntp);
  const indexers = $derived((ui.settings?.indexers ?? []).filter((i) => i.enabled));
  const order = $derived(indexers.map((i) => i.id));
  const active = $derived(selected ? indexers.filter((i) => selected.includes(i.id)) : indexers);
  const allItems = $derived(Object.values(perIndexer).flatMap((r) => r.items ?? []));
  const groups = $derived(sortGroups(groupResults(allItems, ui.settings?.searchDedup ?? true, order), sort));
  const merged = $derived(allItems.length - groups.length);
  const loading = $derived(Object.values(perIndexer).some((r) => r.state === 'loading'));
  const hasMore = $derived(Object.values(perIndexer).some((r) => r.state === 'ok' && r.more));
  const pickerLabel = $derived(
    active.length === indexers.length
      ? t('search.allCount', { n: indexers.length })
      : active.length === 1
        ? active[0].name || t('indexer.indexer')
        : active.length === 0
          ? t('search.none')
          : t('search.someOf', { n: active.length, total: indexers.length })
  );

  $effect(() => {
    const prefs = { category, sort, selected };
    try {
      localStorage.setItem(STORE, JSON.stringify(prefs));
    } catch {}
  });

  function toggleIndexer(id) {
    const cur = selected ?? indexers.map((i) => i.id);
    const next = cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id];
    selected = next.length === indexers.length ? null : next;
  }

  // Agrupa en las estadísticas las peticiones de una misma búsqueda.
  let searchId = null;

  async function runIndexer(ix, gen, offset) {
    perIndexer[ix.id] = { ...(perIndexer[ix.id] ?? { items: [] }), state: 'loading', name: ix.name };
    try {
      const r = await invoke('search_indexer', { id: ix.id, query: lastQuery, category: category || null, offset, searchId });
      if (gen !== generation) return;
      const prev = offset > 0 ? perIndexer[ix.id]?.items ?? [] : [];
      const items = [...prev, ...r.items];
      const limit = ui.settings.searchLimit ?? 100;
      perIndexer[ix.id] = {
        state: 'ok',
        name: ix.name,
        items,
        total: r.total,
        offset,
        ms: r.ms,
        more: r.total != null ? items.length < r.total : r.items.length >= limit,
      };
    } catch (e) {
      if (gen !== generation) return;
      perIndexer[ix.id] = { state: 'error', name: ix.name, items: perIndexer[ix.id]?.items ?? [], error: String(e) };
    }
  }

  function search(e) {
    e?.preventDefault();
    if (!active.length) {
      toast('warn', t('search.selectOne'));
      return;
    }
    generation++;
    searchId = crypto.randomUUID();
    lastQuery = query.trim();
    searched = true;
    expanded = null;
    perIndexer = {};
    for (const ix of active) runIndexer(ix, generation, 0);
  }

  function loadMore() {
    const limit = ui.settings.searchLimit ?? 100;
    for (const ix of active) {
      const r = perIndexer[ix.id];
      if (r?.state === 'ok' && r.more) runIndexer(ix, generation, r.offset + limit);
    }
  }

  function actOf(g) {
    for (const s of g.sources) if (actions[s.link]) return actions[s.link];
    return null;
  }
  function setAct(g, v) {
    for (const s of g.sources) actions[s.link] = v;
  }

  // Prueba cada fuente (indexador) en orden hasta que una funcione.
  async function tryEach(sources, fn) {
    let last;
    for (const src of sources) {
      try {
        return await fn(src);
      } catch (e) {
        last = e;
      }
    }
    throw last;
  }

  async function download(g, sources = g.sources) {
    setAct(g, 'working');
    const s = ui.settings;
    try {
      await tryEach(sources, async (src) => {
        const p = await invoke('inspect_nzb_url', { url: src.link, indexerId: src.indexerId });
        if (s.searchQuickAdd) {
          await invoke('add_job', {
            req: {
              token: p.token,
              name: g.title,
              destDir: s.downloadDir,
              password: p.password ?? null,
              paused: s.addPaused,
              top: false,
              options: { par2: s.par2Enabled, extract: s.extractEnabled, deleteArchives: s.deleteArchives, deletePar2: s.deletePar2, createFolder: s.createJobFolder },
            },
          });
          setAct(g, 'queued');
          toast('success', t('add.queued', { name: g.title }), 3000);
        } else {
          setAct(g, null);
          queueAdd([{ kind: 'url', value: src.link, preview: { ...p, name: g.title } }]);
        }
      });
    } catch (e) {
      setAct(g, 'error');
      toast('error', `${g.title}: ${e}`);
    }
  }

  async function saveNzb(g, sources = g.sources) {
    let dir = ui.settings.nzbSaveDir;
    if (ui.settings.nzbAskSaveDir || !dir) {
      dir = await open({ directory: true, defaultPath: dir || undefined, title: t('common.whereSaveNzb') });
      if (!dir) return;
    }
    setAct(g, 'saving');
    try {
      const path = await tryEach(sources, (src) => invoke('save_nzb_url', { url: src.link, dir, name: g.title, indexerId: src.indexerId }));
      setAct(g, 'saved');
      toast('success', t('common.nzbSaved', { name: path.split(/[\\/]/).pop() }), 3500);
    } catch (e) {
      setAct(g, 'error');
      toast('error', `${g.title}: ${e}`);
    }
  }
</script>

<svelte:window onclick={() => (pickerOpen = false)} />

<div class="view">
  <header class="head">
    <div>
      <h1>{t('nav.search')}</h1>
      <p class="subtitle">
        {#if indexers.length}
          {nntp ? tn('search.subtitleDownload', indexers.length) : tn('search.subtitleSave', indexers.length)}
        {:else}
          {t('search.subtitleEmpty')}
        {/if}
      </p>
    </div>
    <button class="btn" onclick={() => (ui.view = 'indexers')}><Icon name="layers" size={16} /> {t('nav.indexers')}</button>
  </header>

  {#if !indexers.length}
    <div class="empty card" in:fade={{ duration: d(250) }}>
      <div class="orb small"><Icon name="search" size={28} stroke={2} /></div>
      <h3>{t('indexers.emptyTitle')}</h3>
      <p>{t('search.emptyText')}</p>
      <button class="btn primary" onclick={() => (ui.view = 'indexers')}><Icon name="plus" size={16} /> {t('indexers.add')}</button>
    </div>
  {:else}
    <form class="bar" onsubmit={search}>
      <Icon name="search" size={18} />
      <input class="q" placeholder={t('search.placeholder')} bind:value={query} spellcheck="false" />
      <div class="picker-wrap">
        <button type="button" class="pill" class:partial={selected} onclick={(e) => { e.stopPropagation(); pickerOpen = !pickerOpen; }}>
          <Icon name="layers" size={14} /> {pickerLabel} <Icon name="chevron-down" size={13} />
        </button>
        {#if pickerOpen}
          <div class="popover card" transition:scale={{ start: 0.95, duration: d(160), easing: cubicOut }} onclick={(e) => e.stopPropagation()} role="presentation">
            <div class="pop-head">
              <span>{t('search.searchIn')}</span>
              <button type="button" class="link" onclick={() => (selected = null)}>{t('search.all')}</button>
            </div>
            {#each indexers as ix (ix.id)}
              {@const on = !selected || selected.includes(ix.id)}
              <button type="button" class="pop-item" onclick={() => toggleIndexer(ix.id)}>
                <span class="check" class:on>{#if on}<Icon name="check" size={12} stroke={3} />{/if}</span>
                <span class="truncate">{ix.name || ix.url}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
      <select class="select cat" bind:value={category} title={t('search.category')}>
        {#each CATEGORIES as c}<option value={c.id}>{c.label}</option>{/each}
      </select>
      <button class="btn primary" type="submit" disabled={loading && !searched}>
        {#if loading}<Icon name="loader" size={15} class="spin" />{:else}<Icon name="search" size={15} />{/if} {t('nav.search')}
      </button>
    </form>

    {#if searched}
      <div class="status" in:fade={{ duration: d(200) }}>
        {#each active as ix (ix.id)}
          {@const r = perIndexer[ix.id]}
          <span class="ist {r?.state ?? 'loading'}" title={r?.error ?? ''}>
            {#if !r || r.state === 'loading'}
              <Icon name="loader" size={12} class="spin" />
            {:else if r.state === 'ok'}
              <span class="dot ok"></span>
            {:else}
              <Icon name="alert-triangle" size={12} />
            {/if}
            <strong>{ix.name || ix.url}</strong>
            {#if r?.state === 'ok'}
              <span class="muted">{r.total != null && r.total > r.items.length ? t('search.ofTotal', { n: num(r.items.length), total: num(r.total) }) : num(r.items.length)} · {t('time.s', { n: num(r.ms / 1000, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) })}</span>
            {:else if r?.state === 'error'}
              <span class="e truncate">{r.error}</span>
            {/if}
          </span>
        {/each}
      </div>

      <div class="toolbar">
        <span class="count">
          {tn('search.results', groups.length)}
          {#if ui.settings.searchDedup && merged > 0}<span class="chip accent" title={t('search.mergedTitle')}>{tn('search.merged', merged)}</span>{/if}
        </span>
        <select class="select sort" bind:value={sort}>
          {#each SORTS as s}<option value={s.id}>{s.label}</option>{/each}
        </select>
      </div>

      <div class="list">
        {#each groups as g, i (g.key)}
          {@const a = actOf(g)}
          <article class="res card" class:open={expanded === g.key} in:fly={{ y: 10, duration: d(260), delay: d(Math.min(i, 15) * 18), easing: cubicOut }}>
            <div class="r-row">
              <div class="r-main" onclick={() => (expanded = expanded === g.key ? null : g.key)} role="presentation">
                <div class="r-title selectable" title={g.title}>{g.title}</div>
                <div class="r-meta">
                  {#if g.category}<span class="chip muted">{g.category}</span>{/if}
                  <span class="strong">{f.bytes(g.size)}</span>
                  {#if g.date}<span>· {f.ago(g.date)}</span>{/if}
                  {#if g.grabs != null}<span>· {tn('search.grabs', g.grabs)}</span>{/if}
                  {#if g.password}<span class="chip warn" title={t('search.passwordTitle')}><Icon name="lock" size={10} stroke={2.6} /> {t('common.password')}</span>{/if}
                  <span class="ixs">
                    <span class="chip info">{g.indexers[0]}</span>
                    {#if g.indexers.length > 1}<span class="chip accent" title={g.indexers.join(', ')}>+{g.indexers.length - 1}</span>{/if}
                  </span>
                </div>
              </div>
              <div class="r-actions">
                {#if nntp}
                  <button class="btn sm {a === 'queued' ? '' : 'primary'}" onclick={() => download(g)} disabled={a === 'working' || a === 'saving'}>
                    {#if a === 'working'}<Icon name="loader" size={14} class="spin" /> {t('search.adding')}
                    {:else if a === 'queued'}<Icon name="check" size={14} /> {t('status.queued')}
                    {:else}<Icon name="download" size={14} /> {t('common.download')}{/if}
                  </button>
                {/if}
                <button class="btn sm {nntp ? 'ghost' : 'primary'}" onclick={() => saveNzb(g)} disabled={a === 'working' || a === 'saving'} title={t('search.saveNzbTitle')}>
                  {#if a === 'saving'}<Icon name="loader" size={14} class="spin" />
                  {:else if a === 'saved'}<Icon name="check" size={14} />
                  {:else}<Icon name="file" size={14} />{/if}
                  {nntp ? '.nzb' : a === 'saved' ? t('search.saved') : t('common.saveNzb')}
                </button>
                <button class="icon-btn chev" class:rot={expanded === g.key} onclick={() => (expanded = expanded === g.key ? null : g.key)} title={t('common.details')}><Icon name="chevron-down" size={16} /></button>
              </div>
            </div>
            {#if expanded === g.key}
              <div class="sources" transition:slide={{ duration: d(220) }}>
                {#each g.sources as s}
                  <div class="src">
                    <span class="chip info">{s.indexerName}</span>
                    <span class="truncate s-title" title={s.title}>{s.title}</span>
                    <span class="muted">{f.bytes(s.size)}</span>
                    <span class="muted">{s.date ? f.ago(s.date) : ''}</span>
                    <span class="muted">{s.grabs != null ? t('search.grabsShort', { n: num(s.grabs) }) : ''}</span>
                    <span class="s-act">
                      {#if s.details}<button class="icon-btn" title={t('search.openInIndexer')} onclick={() => invoke('open_url', { url: s.details })}><Icon name="external" size={14} /></button>{/if}
                      {#if nntp}<button class="icon-btn" title={t('search.downloadFrom')} onclick={() => download(g, [s])}><Icon name="download" size={14} /></button>{/if}
                      <button class="icon-btn" title={t('search.saveFrom')} onclick={() => saveNzb(g, [s])}><Icon name="file" size={14} /></button>
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </article>
        {:else}
          {#if !loading}
            <div class="empty-inline" in:fade={{ duration: d(200) }}>
              <Icon name="search" size={30} stroke={1.6} />
              <p>{lastQuery ? t('search.noResultsFor', { query: lastQuery }) : t('search.noResults')}</p>
            </div>
          {:else}
            {#each Array(6) as _, i}
              <div class="skeleton card" style="animation-delay:{i * 90}ms"></div>
            {/each}
          {/if}
        {/each}
      </div>

      {#if hasMore && !loading}
        <div class="more"><button class="btn" onclick={loadMore}><Icon name="chevron-down" size={15} /> {t('search.loadMore')}</button></div>
      {/if}
    {:else}
      <div class="hint" in:fade={{ duration: d(250) }}>
        <div class="orb"><div class="orb-ring"></div><Icon name="search" size={30} stroke={2} /></div>
        <h3>{t('search.hintTitle')}</h3>
        <p>
          {active.length === indexers.length ? t('search.hintAll') : tn('search.hintSome', active.length)}
          {#if ui.settings.searchDedup}{t('search.hintDedup')}{/if}
        </p>
        <button class="btn" onclick={() => search()}><Icon name="sparkles" size={15} /> {t('search.latest')}</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .view { padding: 26px 30px 40px; max-width: 1240px; margin: 0 auto; }
  .head { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; }
  h1 { margin: 0; font-family: var(--font-display); font-size: 26px; font-weight: 700; }
  .subtitle { margin: 4px 0 0; color: var(--text-2); }

  .bar {
    margin-top: 20px;
    display: flex; align-items: center; gap: 10px;
    height: 54px; padding: 0 8px 0 18px;
    border-radius: 14px;
    border: 1px solid var(--border-2);
    background: var(--panel);
    color: var(--muted);
    position: relative; z-index: 5;
    transition: border-color 0.2s, box-shadow 0.25s;
  }
  .bar:focus-within { border-color: rgba(var(--accent-rgb), 0.7); box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.12), 0 10px 30px -15px rgba(var(--accent-rgb), 0.7); }
  .q { flex: 1; min-width: 0; height: 100%; border: none; outline: none; background: transparent; color: var(--text); font-size: 14.5px; user-select: text; }
  .q::placeholder { color: var(--muted); }
  .picker-wrap { position: relative; }
  .pill {
    display: inline-flex; align-items: center; gap: 7px;
    height: 36px; padding: 0 12px;
    border-radius: 10px; border: 1px solid var(--border-2);
    background: var(--panel-2); color: var(--text-2);
    font-size: 13px; font-weight: 600; cursor: pointer; white-space: nowrap;
  }
  .pill:hover { background: var(--panel-3); color: var(--text); }
  .pill.partial { border-color: rgba(var(--accent-rgb), 0.45); color: var(--text); }
  .cat { width: 190px; height: 36px; }
  .popover {
    position: absolute; right: 0; top: 44px; z-index: 40;
    width: 250px; padding: 6px;
    max-height: 320px; overflow-y: auto;
    background: var(--panel-2); box-shadow: var(--shadow);
    transform-origin: top right;
  }
  .pop-head { display: flex; justify-content: space-between; align-items: center; padding: 6px 10px 8px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 650; }
  .link { border: none; background: none; color: var(--accent-2); cursor: pointer; font-size: 12px; font-weight: 600; text-transform: none; letter-spacing: 0; }
  .pop-item { width: 100%; height: 34px; display: flex; align-items: center; gap: 10px; padding: 0 10px; border: none; border-radius: 7px; background: transparent; cursor: pointer; font-size: 13px; text-align: left; }
  .pop-item:hover { background: var(--panel-3); }
  .check { width: 18px; height: 18px; border-radius: 5px; border: 1.5px solid var(--border-2); display: grid; place-items: center; flex-shrink: 0; transition: all 0.15s; }
  .check.on { background: var(--grad-diag); border-color: transparent; color: #fff; }

  .status { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 14px; }
  .ist { display: inline-flex; align-items: center; gap: 7px; height: 28px; padding: 0 11px; border-radius: 99px; background: var(--panel); border: 1px solid var(--border); font-size: 12.5px; max-width: 420px; }
  .ist.error { border-color: rgba(248, 113, 113, 0.35); color: #fca5a5; }
  .ist .e { color: #fca5a5; }
  .ist.loading { color: var(--text-2); }

  .toolbar { display: flex; align-items: center; justify-content: space-between; margin: 18px 0 10px; }
  .count { display: flex; align-items: center; gap: 10px; color: var(--text-2); font-weight: 560; }
  .sort { width: 180px; height: 34px; }

  .list { display: flex; flex-direction: column; gap: 8px; }
  .res { padding: 12px 12px 12px 16px; transition: border-color 0.2s, background 0.2s; }
  .res:hover { border-color: var(--border-2); }
  .res.open { border-color: rgba(var(--accent-rgb), 0.35); }
  .r-row { display: flex; align-items: center; gap: 14px; }
  .r-main { flex: 1; min-width: 0; cursor: pointer; }
  .r-title { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .r-meta { display: flex; align-items: center; gap: 7px; margin-top: 5px; font-size: 12.5px; color: var(--text-2); flex-wrap: wrap; }
  .strong { color: var(--text); font-weight: 600; font-variant-numeric: tabular-nums; }
  .ixs { display: inline-flex; gap: 4px; margin-left: 2px; }
  .r-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .chev :global(svg) { transition: transform 0.25s var(--ease); }
  .chev.rot :global(svg) { transform: rotate(180deg); }
  .sources { margin-top: 12px; padding-top: 10px; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 2px; }
  .src { display: grid; grid-template-columns: auto 1fr 76px 90px 72px auto; gap: 10px; align-items: center; padding: 5px 6px; border-radius: 8px; font-size: 12.5px; }
  .src:hover { background: var(--panel-2); }
  .s-title { color: var(--text-2); }
  .s-act { display: flex; gap: 2px; }
  .skeleton { height: 68px; background: linear-gradient(90deg, var(--panel) 0%, var(--panel-2) 50%, var(--panel) 100%); background-size: 200% 100%; animation: shimmer-bg 1.3s ease-in-out infinite; }
  @keyframes shimmer-bg { 0% { background-position: 100% 0; } 100% { background-position: -100% 0; } }
  .more { display: flex; justify-content: center; margin-top: 16px; }

  .empty, .hint { text-align: center; padding: 50px 20px; display: flex; flex-direction: column; align-items: center; gap: 6px; margin-top: 20px; }
  .empty h3, .hint h3 { margin: 12px 0 0; font-family: var(--font-display); font-size: 19px; }
  .empty p, .hint p { margin: 4px 0 14px; color: var(--text-2); max-width: 460px; line-height: 1.5; }
  .empty-inline { text-align: center; padding: 40px; color: var(--muted); display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .orb {
    position: relative; width: 84px; height: 84px; border-radius: 50%;
    display: grid; place-items: center; color: #fff;
    background: radial-gradient(circle at 30% 25%, rgba(255, 255, 255, 0.35), transparent 45%), var(--grad-diag);
    box-shadow: 0 20px 60px -15px rgba(var(--accent-rgb), 0.8);
    animation: float 4s ease-in-out infinite;
  }
  .orb.small { width: 64px; height: 64px; }
  .orb-ring { position: absolute; inset: -12px -26px; border-radius: 50%; border: 2px solid rgba(var(--accent-rgb), 0.45); transform: rotate(-18deg); animation: ring 6s linear infinite; }
  @keyframes float { 50% { transform: translateY(-8px); } }
  @keyframes ring { to { transform: rotate(342deg); } }
</style>
