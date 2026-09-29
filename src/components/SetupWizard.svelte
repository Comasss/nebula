<script>
  import { open } from '@tauri-apps/plugin-dialog';
  import { fly, fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import ServerForm from './ServerForm.svelte';
  import IndexerForm from './IndexerForm.svelte';
  import SettingRow from './SettingRow.svelte';
  import logo from '../assets/logo.svg';
  import { ui, saveSettings, toast, d } from '../lib/state.svelte.js';
  import { blankIndexer } from '../lib/search.js';

  const s0 = $state.snapshot(ui.settings);
  const firstTime = !s0.firstRunDone;
  let dir = $state(1);
  let saving = $state(false);

  let features = $state({
    nntp: s0.featuresChosen ? s0.featureNntp : true,
    indexers: s0.featuresChosen ? s0.featureIndexers : false,
  });
  let server = $state(
    s0.servers[0]
      ? structuredClone(s0.servers[0])
      : {
          id: crypto.randomUUID().replaceAll('-', ''),
          name: '', host: '', port: 563, ssl: true, verifyCert: true, username: '', password: '', passwordEnc: '',
          connections: 20, priority: 0, enabled: true, retentionDays: 0,
        }
  );
  let indexer = $state(s0.indexers[0] ? structuredClone(s0.indexers[0]) : blankIndexer());
  let prefs = $state({
    downloadDir: s0.downloadDir,
    nzbSaveDir: s0.nzbSaveDir,
    askDestination: s0.askDestination,
    par2Enabled: s0.par2Enabled,
    extractEnabled: s0.extractEnabled,
    clipboardMonitor: s0.clipboardMonitor,
    associateNzb: s0.associateNzb || firstTime,
    closeToTray: s0.closeToTray,
    searchDedup: s0.searchDedup,
    searchQuickAdd: s0.searchQuickAdd,
  });

  const LABELS = { welcome: 'Bienvenida', features: 'Funciones', indexers: 'Indexadores', server: 'Servidor', prefs: 'Preferencias', done: 'Listo' };
  const steps = $derived(
    [firstTime && 'welcome', 'features', features.indexers && 'indexers', features.nntp && 'server', 'prefs', 'done'].filter(Boolean)
  );
  let current = $state(firstTime ? 'welcome' : 'features');
  const idx = $derived(steps.indexOf(current));

  const mode = $derived(
    features.nntp && features.indexers ? 'both' : features.indexers ? 'indexers' : features.nntp ? 'nntp' : 'none'
  );
  const MODE_TEXT = {
    both: 'Buscarás en tus indexadores y descargarás directamente con Nebula, con un clic.',
    indexers: 'Buscarás en tus indexadores y guardarás los .nzb en tu PC para usarlos con otro programa.',
    nntp: 'Añadirás .nzb desde archivos, enlaces o el portapapeles y Nebula los descargará de Usenet.',
    none: 'Activa al menos una función para continuar.',
  };

  let skipServer = $state(false);
  let skipIndexer = $state(false);

  function go(step) {
    dir = steps.indexOf(step) > idx ? 1 : -1;
    current = step;
  }
  const next = () => go(steps[idx + 1]);
  const back = () => go(steps[idx - 1]);

  function nextFromFeatures() {
    if (mode === 'none') {
      toast('warn', 'Activa al menos una función.');
      return;
    }
    next();
  }

  function nextFromServer() {
    if (!server.host.trim()) {
      toast('error', 'Indica la dirección del servidor NNTP (o pulsa «Omitir»).');
      return;
    }
    if (!server.username.trim()) toast('warn', 'No has puesto usuario: la mayoría de proveedores lo requieren.', 4000);
    skipServer = false;
    next();
  }

  function nextFromIndexer() {
    if (!/^https?:\/\//i.test(indexer.url.trim())) {
      toast('error', 'Indica la URL del indexador (o pulsa «Omitir»).');
      return;
    }
    if (!indexer.apiKey.trim()) toast('warn', 'Sin API key la mayoría de indexadores no devuelven resultados.', 4000);
    skipIndexer = false;
    next();
  }

  async function browse(key) {
    const p = await open({ directory: true, defaultPath: prefs[key] });
    if (p) prefs[key] = p;
  }

  async function finish() {
    saving = true;
    const servers = $state.snapshot(ui.settings.servers);
    if (features.nntp && !skipServer && server.host.trim()) {
      const item = $state.snapshot(server);
      item.port = Number(item.port) || (item.ssl ? 563 : 119);
      const i = servers.findIndex((x) => x.id === item.id);
      if (i >= 0) servers[i] = item;
      else servers.push(item);
    }
    const indexers = $state.snapshot(ui.settings.indexers);
    if (features.indexers && !skipIndexer && indexer.url.trim()) {
      const item = $state.snapshot(indexer);
      item.url = item.url.trim().replace(/\/+$/, '');
      if (!item.name.trim()) item.name = item.url.replace(/^https?:\/\/(api\.)?/i, '').split(/[/.]/)[0];
      const i = indexers.findIndex((x) => x.id === item.id);
      if (i >= 0) indexers[i] = item;
      else indexers.push(item);
    }
    const ok = await saveSettings({
      ...$state.snapshot(prefs),
      servers,
      indexers,
      featureNntp: features.nntp,
      featureIndexers: features.indexers,
      featuresChosen: true,
      firstRunDone: true,
    });
    saving = false;
    if (ok) {
      ui.setupOpen = false;
      ui.view = features.indexers ? 'search' : 'downloads';
    }
  }
</script>

<div class="overlay" transition:fade={{ duration: d(220) }}>
  <div class="wizard" in:scale={{ start: 0.95, duration: d(320), easing: cubicOut }}>
    <div class="glow"></div>
    <div class="steps">
      {#each steps as st, i (st)}
        <div class="stp" class:done={i < idx} class:cur={i === idx}>
          <span class="num">{#if i < idx}<Icon name="check" size={12} stroke={3} />{:else}{i + 1}{/if}</span>
          <span class="lbl">{LABELS[st]}</span>
        </div>
        {#if i < steps.length - 1}<div class="line" class:done={i < idx}></div>{/if}
      {/each}
    </div>

    <div class="stage">
      {#key current}
        <div class="page" in:fly={{ x: 40 * dir, duration: d(360), easing: cubicOut }} out:fly={{ x: -40 * dir, duration: d(200) }}>
          {#if current === 'welcome'}
            <div class="welcome">
              <img class="hero" src={logo} alt="" />
              <h1>Bienvenido a <span class="grad-text">Nebula</span></h1>
              <p>Busca en tus indexadores y descarga desde Usenet con verificación, reparación y extracción automáticas. Lo dejamos listo en un minuto.</p>
              <div class="features-list">
                <div><Icon name="search" size={16} /> Búsqueda en varios indexadores</div>
                <div><Icon name="zap" size={16} /> Multiconexión y servidores de respaldo</div>
                <div><Icon name="shield" size={16} /> Reparación PAR2 integrada</div>
                <div><Icon name="archive" size={16} /> Extracción RAR/7z/ZIP incluida</div>
              </div>
            </div>
          {:else if current === 'features'}
            <h2>{firstTime ? '¿Qué quieres usar?' : 'Novedad: búsqueda en indexadores'}</h2>
            <p class="lead">Activa las funciones que necesites (al menos una). Puedes cambiarlo cuando quieras en Configuración.</p>
            <div class="fcards">
              <button type="button" class="fcard" class:on={features.indexers} onclick={() => (features.indexers = !features.indexers)}>
                <div class="f-top">
                  <div class="f-icon"><Icon name="search" size={22} /></div>
                  <span class="f-check">{#if features.indexers}<span in:scale={{ duration: d(200) }}><Icon name="check" size={14} stroke={3} /></span>{/if}</span>
                </div>
                <div class="f-title">Indexadores</div>
                <div class="f-desc">Busca en NZBGeek, NZBFinder, DrunkenSlug y otros indexadores Newznab a la vez, sin duplicados.</div>
              </button>
              <button type="button" class="fcard" class:on={features.nntp} onclick={() => (features.nntp = !features.nntp)}>
                <div class="f-top">
                  <div class="f-icon"><Icon name="server" size={22} /></div>
                  <span class="f-check">{#if features.nntp}<span in:scale={{ duration: d(200) }}><Icon name="check" size={14} stroke={3} /></span>{/if}</span>
                </div>
                <div class="f-title">Descargas Usenet (NNTP)</div>
                <div class="f-desc">Descarga, repara y extrae los NZB con las credenciales de tu proveedor Usenet.</div>
              </button>
            </div>
            <div class="mode alert {mode === 'none' ? 'warn' : 'info'}">
              <Icon name={mode === 'none' ? 'alert-triangle' : 'info'} size={16} />
              {#key mode}<span in:fade={{ duration: d(200) }}>{MODE_TEXT[mode]}</span>{/key}
            </div>
          {:else if current === 'indexers'}
            <h2>Tu indexador</h2>
            <p class="lead">Elige tu indexador y pega la API key de tu perfil. Podrás añadir más en la pestaña Indexadores.</p>
            <IndexerForm bind:indexer compact />
          {:else if current === 'server'}
            <h2>Tu servidor Usenet</h2>
            <p class="lead">Introduce los datos NNTP que te dio tu proveedor. Podrás añadir servidores de respaldo más tarde.</p>
            <ServerForm bind:server compact />
            <div class="alert info wz-debrid">
              <Icon name="cloud" size={15} />
              <span>¿Usas <strong>TorBox</strong> (plan Pro) o <strong>Premiumize</strong>? Pulsa «Omitir» si no tienes proveedor Usenet y añade tu API key después en la pestaña <strong>Debrid</strong>.</span>
            </div>
          {:else if current === 'prefs'}
            <h2>Preferencias</h2>
            <p class="lead">Puedes cambiar todo esto cuando quieras en Configuración.</p>
            {#if features.nntp}
              <div class="field">
                <label for="wz-dir">Carpeta de descargas</label>
                <div class="dirrow">
                  <input id="wz-dir" class="input" bind:value={prefs.downloadDir} />
                  <button class="btn" onclick={() => browse('downloadDir')}>Examinar…</button>
                </div>
              </div>
            {:else}
              <div class="field">
                <label for="wz-nzb">Carpeta donde guardar los .nzb</label>
                <div class="dirrow">
                  <input id="wz-nzb" class="input" bind:value={prefs.nzbSaveDir} />
                  <button class="btn" onclick={() => browse('nzbSaveDir')}>Examinar…</button>
                </div>
                <span class="hint">Puedes usar la carpeta vigilada de otro programa para que los recoja automáticamente.</span>
              </div>
            {/if}
            <div class="card prefs">
              {#if features.indexers}
                <SettingRow label="Eliminar duplicados entre indexadores" description="El mismo release subido a varios indexadores aparece una sola vez." checked={prefs.searchDedup} onchange={(v) => (prefs.searchDedup = v)} />
              {/if}
              {#if features.indexers && features.nntp}
                <SettingRow label="Descargar con un clic desde la búsqueda" description="Añade el resultado directamente a la cola, sin ventanas intermedias." checked={prefs.searchQuickAdd} onchange={(v) => (prefs.searchQuickAdd = v)} />
              {/if}
              {#if features.nntp}
                <SettingRow label="Preguntar dónde guardar cada NZB añadido a mano" checked={prefs.askDestination} onchange={(v) => (prefs.askDestination = v)} />
                <SettingRow label="Verificar y reparar con PAR2" checked={prefs.par2Enabled} onchange={(v) => (prefs.par2Enabled = v)} />
                <SettingRow label="Extraer archivos comprimidos" checked={prefs.extractEnabled} onchange={(v) => (prefs.extractEnabled = v)} />
                <SettingRow label="Abrir archivos .nzb con Nebula" checked={prefs.associateNzb} onchange={(v) => (prefs.associateNzb = v)} />
              {/if}
              <SettingRow label="Detectar enlaces .nzb en el portapapeles" checked={prefs.clipboardMonitor} onchange={(v) => (prefs.clipboardMonitor = v)} />
              <SettingRow label="Seguir en la bandeja al cerrar la ventana" checked={prefs.closeToTray} onchange={(v) => (prefs.closeToTray = v)} />
            </div>
          {:else}
            <div class="welcome">
              <div class="done-badge"><Icon name="check" size={40} stroke={2.6} /></div>
              <h1>¡Todo listo!</h1>
              <p>
                {#if features.indexers}Ve a <strong>Buscar</strong> para encontrar lo que quieras en {skipIndexer || !indexer.url.trim() ? 'tus indexadores' : indexer.name || 'tu indexador'}.{/if}
                {#if features.nntp && server.host.trim() && !skipServer}Descargaremos desde <strong>{server.host}</strong> con {server.connections} conexiones.{/if}
                {#if !features.indexers}Arrastra un .nzb a la ventana, pega un enlace o cópialo al portapapeles para empezar.{/if}
              </p>
            </div>
          {/if}
        </div>
      {/key}
    </div>

    <footer>
      {#if current === 'server'}
        <button class="btn ghost" onclick={() => { skipServer = true; next(); }}>Omitir</button>
      {:else if current === 'indexers'}
        <button class="btn ghost" onclick={() => { skipIndexer = true; next(); }}>Omitir</button>
      {/if}
      <div class="spacer"></div>
      {#if idx > 0}
        <button class="btn" onclick={back}><Icon name="chevron-left" size={16} /> Atrás</button>
      {/if}
      {#if current === 'welcome'}
        <button class="btn primary lg" onclick={next}>Empezar <Icon name="chevron-right" size={16} /></button>
      {:else if current === 'features'}
        <button class="btn primary" onclick={nextFromFeatures} disabled={mode === 'none'}>Siguiente <Icon name="chevron-right" size={16} /></button>
      {:else if current === 'indexers'}
        <button class="btn primary" onclick={nextFromIndexer}>Siguiente <Icon name="chevron-right" size={16} /></button>
      {:else if current === 'server'}
        <button class="btn primary" onclick={nextFromServer}>Siguiente <Icon name="chevron-right" size={16} /></button>
      {:else if current === 'prefs'}
        <button class="btn primary" onclick={next}>Siguiente <Icon name="chevron-right" size={16} /></button>
      {:else}
        <button class="btn primary lg" onclick={finish} disabled={saving}><Icon name="rocket" size={16} /> Empezar</button>
      {/if}
    </footer>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 40px 0 0 0; z-index: 200; display: grid; place-items: center; background: rgba(5, 6, 10, 0.75); backdrop-filter: blur(10px); padding: 24px; }
  .wizard { position: relative; width: min(780px, 100%); max-height: calc(100vh - 90px); display: flex; flex-direction: column; background: var(--panel); border: 1px solid var(--border-2); border-radius: 22px; box-shadow: var(--shadow); overflow: hidden; }
  .glow { position: absolute; inset: -200px -100px auto auto; width: 500px; height: 400px; background: radial-gradient(circle, rgba(var(--accent-rgb), 0.25), transparent 65%); pointer-events: none; }
  .wz-debrid { margin-top: 14px; }
  .steps { position: relative; display: flex; align-items: center; gap: 10px; padding: 22px 28px 0; }
  .stp { display: flex; align-items: center; gap: 8px; font-size: 12.5px; color: var(--muted); font-weight: 600; white-space: nowrap; }
  .num { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; font-size: 11.5px; font-weight: 700; background: var(--panel-3); color: var(--text-2); transition: all 0.3s var(--ease); }
  .stp.cur { color: var(--text); }
  .stp.cur .num { background: var(--grad-diag); color: #fff; box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.2); }
  .stp.done .num { background: rgba(52, 211, 153, 0.18); color: var(--ok); }
  .line { flex: 1; min-width: 12px; height: 2px; border-radius: 2px; background: var(--panel-3); position: relative; overflow: hidden; }
  .line::after { content: ''; position: absolute; inset: 0; background: var(--grad); transform: scaleX(0); transform-origin: left; transition: transform 0.5s var(--ease); }
  .line.done::after { transform: scaleX(1); }
  .stage { position: relative; display: grid; overflow-y: auto; overflow-x: hidden; min-height: 420px; }
  .page { grid-area: 1 / 1; padding: 26px 28px 20px; }
  .welcome { text-align: center; padding: 16px 20px 0; }
  .hero { width: 110px; height: 110px; animation: float 4s ease-in-out infinite; filter: drop-shadow(0 18px 40px rgba(var(--accent-rgb), 0.5)); }
  @keyframes float { 50% { transform: translateY(-8px) rotate(-3deg); } }
  .welcome h1 { font-family: var(--font-display); font-size: 28px; margin: 18px 0 8px; }
  .welcome p { color: var(--text-2); max-width: 500px; margin: 0 auto; line-height: 1.55; }
  .features-list { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; max-width: 540px; margin: 26px auto 0; text-align: left; }
  .features-list div { display: flex; align-items: center; gap: 10px; padding: 11px 14px; border-radius: 12px; background: var(--panel-2); border: 1px solid var(--border); font-size: 13px; color: var(--text-2); }
  .features-list :global(svg) { color: var(--accent-2); }
  h2 { font-family: var(--font-display); font-size: 21px; margin: 0; }
  .lead { color: var(--text-2); margin: 6px 0 20px; }
  .fcards { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .fcard {
    text-align: left; padding: 18px; border-radius: 16px;
    border: 1.5px solid var(--border-2); background: var(--panel-2);
    cursor: pointer; display: flex; flex-direction: column; gap: 6px;
    transition: border-color 0.25s, background 0.25s, transform 0.2s var(--ease), box-shadow 0.3s;
  }
  .fcard:hover { transform: translateY(-2px); border-color: #3a4156; }
  .fcard.on { border-color: rgba(var(--accent-rgb), 0.75); background: linear-gradient(160deg, rgba(var(--accent-rgb), 0.16), transparent 70%), var(--panel-2); box-shadow: 0 14px 40px -20px rgba(var(--accent-rgb), 0.9); }
  .f-top { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 6px; }
  .f-icon { width: 44px; height: 44px; border-radius: 12px; display: grid; place-items: center; background: var(--panel-3); color: var(--text-2); transition: all 0.3s; }
  .fcard.on .f-icon { background: var(--grad-diag); color: #fff; }
  .f-check { width: 24px; height: 24px; border-radius: 50%; border: 1.5px solid var(--border-2); display: grid; place-items: center; color: #fff; transition: all 0.2s; }
  .fcard.on .f-check { background: var(--grad-diag); border-color: transparent; }
  .f-title { font-weight: 650; font-size: 15px; }
  .f-desc { font-size: 12.5px; color: var(--text-2); line-height: 1.5; }
  .mode { margin-top: 16px; }
  .dirrow { display: flex; gap: 8px; }
  .prefs { margin-top: 16px; overflow: hidden; }
  .done-badge { width: 86px; height: 86px; margin: 20px auto 0; border-radius: 50%; display: grid; place-items: center; background: linear-gradient(135deg, #10b981, #34d399); color: #fff; box-shadow: 0 16px 50px -12px rgba(52, 211, 153, 0.8); animation: pop-in 0.6s var(--ease); }
  @keyframes pop-in { 0% { transform: scale(0.3); opacity: 0; } 60% { transform: scale(1.08); } }
  footer { display: flex; align-items: center; gap: 10px; padding: 16px 28px; border-top: 1px solid var(--border); background: rgba(0, 0, 0, 0.15); }
</style>
