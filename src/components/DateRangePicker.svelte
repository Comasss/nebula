<script>
  import { t, locale, i18n } from '../lib/i18n.svelte.js';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { d } from '../lib/state.svelte.js';

  // Selector de rango de fechas con calendario propio. `from` y `to` son 'AAAA-MM-DD'.
  let { from = $bindable(''), to = $bindable('') } = $props();

  let open = $state(false);
  let anchor = $state(null); // primer día elegido mientras se escoge el segundo
  let hover = $state(null);
  let view = $state(monthStart(new Date())); // mes de la derecha
  let root;
  let alignRight = $state(false);

  const pad = (n) => String(n).padStart(2, '0');
  const ymd = (dt) => `${dt.getFullYear()}-${pad(dt.getMonth() + 1)}-${pad(dt.getDate())}`;
  const parse = (s) => {
    const [y, m, dd] = (s || '').split('-').map(Number);
    return y && m && dd ? new Date(y, m - 1, dd) : null;
  };
  function monthStart(dt) {
    return new Date(dt.getFullYear(), dt.getMonth(), 1);
  }
  const addMonths = (dt, n) => new Date(dt.getFullYear(), dt.getMonth() + n, 1);
  const today = ymd(new Date());

  // Primer día de la semana según el idioma (lunes salvo en inglés de EE. UU. y otros que empiezan en domingo).
  const firstDay = $derived.by(() => {
    i18n.lang;
    try {
      const info = new Intl.Locale(locale());
      const wi = info.getWeekInfo?.() ?? info.weekInfo;
      if (wi?.firstDay) return wi.firstDay % 7;
    } catch {}
    return ['en', 'ja', 'zh', 'hi', 'pt', 'ar'].includes(i18n.lang) ? 0 : 1;
  });
  const weekdays = $derived.by(() => {
    const fmt = new Intl.DateTimeFormat(locale(), { weekday: 'narrow' });
    // 2023-01-01 fue domingo.
    return Array.from({ length: 7 }, (_, i) => fmt.format(new Date(2023, 0, 1 + ((firstDay + i) % 7))));
  });
  const monthName = (dt) => {
    const s = new Intl.DateTimeFormat(locale(), { month: 'long', year: 'numeric' }).format(dt);
    return s.charAt(0).toLocaleUpperCase(locale()) + s.slice(1);
  };
  const label = (s) => {
    const dt = parse(s);
    return dt ? dt.toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' }) : '—';
  };

  function weeks(month) {
    const lead = (month.getDay() - firstDay + 7) % 7;
    const days = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate();
    const cells = [];
    for (let i = 0; i < lead; i++) cells.push(null);
    for (let n = 1; n <= days; n++) cells.push(ymd(new Date(month.getFullYear(), month.getMonth(), n)));
    while (cells.length % 7) cells.push(null);
    return cells;
  }
  const months = $derived([addMonths(view, -1), view]);
  const canNext = $derived(ymd(addMonths(view, 1)) <= today);

  // Rango que se ve resaltado (el elegido o el que se está eligiendo).
  const range = $derived.by(() => {
    if (anchor) {
      const other = hover ?? anchor;
      return anchor <= other ? [anchor, other] : [other, anchor];
    }
    return from && to ? [from, to] : [null, null];
  });

  function toggle() {
    open = !open;
    if (open) {
      // El calendario se abre hacia el lado donde hay sitio.
      const r = root.getBoundingClientRect();
      alignRight = r.left + 600 > window.innerWidth && r.right - 600 > 0;
      anchor = null;
      hover = null;
      const end = parse(to) ?? new Date();
      view = monthStart(end);
    }
  }

  function choose(day) {
    if (day > today) return;
    if (!anchor) {
      anchor = day;
      return;
    }
    const [a, b] = anchor <= day ? [anchor, day] : [day, anchor];
    from = a;
    to = b;
    anchor = null;
    open = false;
  }

  function onKey(e) {
    if (open && e.key === 'Escape') {
      open = false;
      anchor = null;
    }
  }
  function onDoc(e) {
    if (open && root && !root.contains(e.target)) {
      open = false;
      anchor = null;
    }
  }
</script>

<svelte:window onkeydown={onKey} onmousedown={onDoc} />

<div class="drp" bind:this={root}>
  <button type="button" class="trigger" class:open onclick={toggle} aria-expanded={open}>
    <Icon name="calendar" size={15} />
    <span class="v">{label(from)}</span>
    <Icon name="chevron-right" size={13} class="sep" />
    <span class="v">{label(to)}</span>
    <Icon name="chevron-down" size={14} class="caret" />
  </button>

  {#if open}
    <div class="pop card" class:right={alignRight} transition:fly={{ y: -6, duration: d(200), easing: cubicOut }} role="dialog">
      <div class="pop-head">
        <button type="button" class="icon-btn" onclick={() => (view = addMonths(view, -1))} aria-label="‹"><Icon name="chevron-left" size={16} /></button>
        <span class="hint-t">{anchor ? t('stats.pickEnd') : t('stats.pickStart')}</span>
        <button type="button" class="icon-btn" disabled={!canNext} onclick={() => (view = addMonths(view, 1))} aria-label="›"><Icon name="chevron-right" size={16} /></button>
      </div>
      <div class="months">
        {#each months as m (ymd(m))}
          <div class="month">
            <div class="m-title">{monthName(m)}</div>
            <div class="grid">
              {#each weekdays as w, i (i)}<span class="wd">{w}</span>{/each}
              {#each weeks(m) as day, i (day ?? `e${i}`)}
                {#if day}
                  {@const inR = range[0] && day >= range[0] && day <= range[1]}
                  {@const start = day === range[0]}
                  {@const end = day === range[1]}
                  <button
                    type="button"
                    class="day"
                    class:in={inR}
                    class:start
                    class:end
                    class:today={day === today}
                    class:col0={i % 7 === 0}
                    class:col6={i % 7 === 6}
                    disabled={day > today}
                    onclick={() => choose(day)}
                    onmouseenter={() => (hover = day)}>
                    <span>{Number(day.slice(8))}</span>
                  </button>
                {:else}
                  <span></span>
                {/if}
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .drp { position: relative; }
  .trigger {
    height: 38px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-radius: 11px;
    border: 1px solid var(--border-2);
    background: var(--panel);
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.18s, box-shadow 0.18s, background 0.18s;
  }
  .trigger:hover { background: var(--panel-2); border-color: #394058; }
  .trigger.open { border-color: rgba(var(--accent-rgb), 0.85); box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.18); }
  .trigger :global(svg) { color: var(--accent-2); }
  .trigger :global(.sep), .trigger :global(.caret) { color: var(--muted); }
  .trigger :global(.caret) { margin-left: 2px; transition: transform 0.25s var(--ease); }
  .trigger.open :global(.caret) { transform: rotate(180deg); }
  .v { font-variant-numeric: tabular-nums; white-space: nowrap; }

  .pop {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    z-index: 30;
    padding: 12px 14px 14px;
    box-shadow: var(--shadow);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.025), rgba(255, 255, 255, 0)), var(--panel-2);
    border-color: var(--border-2);
  }
  .pop.right { left: auto; right: 0; }
  .pop-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 6px; }
  .hint-t { font-size: 12px; color: var(--text-2); }
  .months { display: flex; gap: 22px; }
  .month { width: 252px; }
  .m-title { text-align: center; font-family: var(--font-display); font-weight: 650; font-size: 14px; margin-bottom: 8px; }
  .grid { display: grid; grid-template-columns: repeat(7, 36px); row-gap: 3px; }
  .wd { height: 26px; display: grid; place-items: center; font-size: 11px; font-weight: 650; color: var(--muted); text-transform: uppercase; }
  .day {
    position: relative;
    height: 34px;
    border: none;
    padding: 0;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .day span {
    position: relative;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    margin: 1px auto;
    border-radius: 9px;
    transition: background 0.15s, color 0.15s;
  }
  .day:hover:not(:disabled) span { background: var(--panel-4); }
  .day.today span { box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.7); }
  .day.in { background: rgba(var(--accent-rgb), 0.14); }
  .day.in.col0, .day.start { border-top-left-radius: 9px; border-bottom-left-radius: 9px; }
  .day.in.col6, .day.end { border-top-right-radius: 9px; border-bottom-right-radius: 9px; }
  .day.start span, .day.end span, .day.start:hover:not(:disabled) span, .day.end:hover:not(:disabled) span {
    background: var(--grad-diag);
    color: #fff;
    font-weight: 700;
    box-shadow: 0 4px 14px -4px rgba(var(--accent-rgb), 0.9);
  }
  .day:disabled { color: var(--muted); opacity: 0.4; cursor: default; }
</style>
