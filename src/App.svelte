<script>
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import TitleBar from './components/TitleBar.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Toasts from './components/Toasts.svelte';
  import Dialog from './components/Dialog.svelte';
  import AddDialog from './components/AddDialog.svelte';
  import SetupWizard from './components/SetupWizard.svelte';
  import ClipboardPrompt from './components/ClipboardPrompt.svelte';
  import DropOverlay from './components/DropOverlay.svelte';
  import Downloads from './views/Downloads.svelte';
  import History from './views/History.svelte';
  import Servers from './views/Servers.svelte';
  import Settings from './views/Settings.svelte';
  import Logs from './views/Logs.svelte';
  import Search from './views/Search.svelte';
  import Indexers from './views/Indexers.svelte';
  import Debrid from './views/Debrid.svelte';
  import { ui, init, queueAdd, toast, d } from './lib/state.svelte.js';

  const VIEWS = { search: Search, downloads: Downloads, history: History, indexers: Indexers, servers: Servers, debrid: Debrid, settings: Settings, logs: Logs };
  const ORDER = ['search', 'downloads', 'history', 'indexers', 'servers', 'debrid', 'settings', 'logs'];
  const NEEDS = { search: 'featureIndexers', indexers: 'featureIndexers', downloads: 'featureNntp', history: 'featureNntp', servers: 'featureNntp', debrid: 'featureNntp' };

  // Si se desactiva la función de la vista actual, se salta a la primera disponible.
  $effect(() => {
    const s = ui.settings;
    if (!s) return;
    const need = NEEDS[ui.view];
    if (need && !s[need]) ui.view = s.featureNntp ? 'downloads' : 'search';
  });
  let prevIndex = 0;
  let direction = $state(1);

  $effect(() => {
    const i = ORDER.indexOf(ui.view);
    direction = i >= prevIndex ? 1 : -1;
    prevIndex = i;
  });

  $effect(() => {
    if (!ui.settings) return;
    document.documentElement.dataset.accent = ui.settings.accent || 'nebula';
    document.documentElement.dataset.anim = ui.settings.animations ? 'on' : 'off';
  });

  onMount(() => {
    let unlistenDrop;
    init().catch((e) => {
      console.error(e);
      toast('error', `Error al iniciar: ${e}`);
    });
    getCurrentWebview()
      .onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === 'enter' || p.type === 'over') {
          ui.dragging = true;
        } else if (p.type === 'leave') {
          ui.dragging = false;
        } else if (p.type === 'drop') {
          ui.dragging = false;
          const nzbs = (p.paths ?? []).filter((x) => /\.nzb(\.gz)?$/i.test(x));
          if (nzbs.length) queueAdd(nzbs.map((value) => ({ kind: 'file', value })));
          else toast('warn', 'Solo se pueden añadir archivos .nzb');
        }
      })
      .then((f) => (unlistenDrop = f));

    const onKey = (e) => {
      if (e.ctrlKey && e.key.toLowerCase() === 'r') e.preventDefault();
      if (e.key === 'F5') e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => {
      unlistenDrop?.();
      window.removeEventListener('keydown', onKey);
    };
  });

  const View = $derived(VIEWS[ui.view] ?? Downloads);
</script>

<div class="app">
  <TitleBar />
  <div class="body">
    <Sidebar />
    <main>
      {#if ui.ready}
        {#key ui.view}
          <div class="page" in:fly={{ y: 14 * direction, duration: d(320), easing: cubicOut, opacity: 0 }}>
            <View />
          </div>
        {/key}
      {:else}
        <div class="boot"><div class="spinner"></div></div>
      {/if}
    </main>
  </div>
</div>

{#if ui.ready}
  <ClipboardPrompt />
  <AddDialog />
  {#if ui.setupOpen && ui.settings}
    <SetupWizard />
  {/if}
  <Dialog />
{/if}
<DropOverlay />
<Toasts />

<style>
  .app { height: 100%; display: flex; flex-direction: column; }
  .body { flex: 1; display: flex; min-height: 0; }
  main {
    flex: 1;
    min-width: 0;
    position: relative;
    display: grid;
    background:
      radial-gradient(900px 500px at 85% -10%, rgba(var(--accent-rgb), 0.09), transparent 60%),
      radial-gradient(700px 400px at 0% 110%, rgba(var(--accent-rgb), 0.05), transparent 60%),
      var(--bg-2);
    border-top-left-radius: 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
    border-left: 1px solid rgba(255, 255, 255, 0.04);
    overflow: hidden;
  }
  .page { grid-area: 1 / 1; overflow-y: auto; min-height: 0; height: 100%; }
  .boot { display: grid; place-items: center; }
  .spinner { width: 34px; height: 34px; border-radius: 50%; border: 3px solid var(--panel-3); border-top-color: var(--accent); animation: spin 0.8s linear infinite; }
</style>
