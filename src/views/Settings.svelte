<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { fly, scale } from 'svelte/transition';
  import Icon from '../components/Icon.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import { ui, saveSettings, toast, d } from '../lib/state.svelte.js';
  import { usableDebrid, debridLabel, hasNntp } from '../lib/debrid.js';

  const s = $derived(ui.settings);
  const debridList = $derived(usableDebrid(s));
  let newPattern = $state('');

  const SECTIONS = [
    { id: 'features', label: 'Funciones', icon: 'sparkles' },
    { id: 'debrid', label: 'NNTP / Debrid', icon: 'cloud' },
    { id: 'search', label: 'Búsqueda', icon: 'search' },
    { id: 'folders', label: 'Carpetas', icon: 'folder' },
    { id: 'post', label: 'Post-procesado', icon: 'package' },
    { id: 'links', label: 'Enlaces y portapapeles', icon: 'clipboard' },
    { id: 'windows', label: 'Windows', icon: 'monitor' },
    { id: 'network', label: 'Conexión', icon: 'wifi' },
    { id: 'look', label: 'Apariencia', icon: 'palette' },
    { id: 'about', label: 'Acerca de', icon: 'info' },
  ];
  let section = $state('features');

  function setFeature(key, v) {
    const other = key === 'featureNntp' ? 'featureIndexers' : 'featureNntp';
    if (!v && !s[other]) {
      toast('warn', 'Debe quedar al menos una función activa.');
      return;
    }
    saveSettings({ [key]: v });
  }

  const ACCENTS = [
    { id: 'nebula', name: 'Nebula', a: '#7c5cff', b: '#22d3ee' },
    { id: 'ocean', name: 'Océano', a: '#3b82f6', b: '#2dd4bf' },
    { id: 'emerald', name: 'Esmeralda', a: '#10b981', b: '#a3e635' },
    { id: 'sunset', name: 'Atardecer', a: '#f97316', b: '#ec4899' },
    { id: 'rose', name: 'Rosa', a: '#ec4899', b: '#8b5cf6' },
  ];

  function set(key) {
    return (v) => saveSettings({ [key]: v });
  }

  async function pick(key) {
    const dir = await open({ directory: true, defaultPath: s[key] || undefined });
    if (dir) saveSettings({ [key]: dir });
  }

  function num(key, min, max) {
    return (e) => {
      let v = Math.round(Number(String(e.target.value).replace(',', '.')));
      if (isNaN(v)) v = min;
      v = Math.max(min, Math.min(max, v));
      e.target.value = v;
      saveSettings({ [key]: v });
    };
  }

  function addPattern() {
    const p = newPattern.trim();
    if (!p || s.clipboardPatterns.includes(p)) return;
    saveSettings({ clipboardPatterns: [...s.clipboardPatterns, p] });
    newPattern = '';
  }

  async function resetPatterns() {
    saveSettings({ clipboardPatterns: await invoke('default_patterns') });
  }

  function scrollTo(id) {
    section = id;
    document.getElementById(`sec-${id}`)?.scrollIntoView({ behavior: s.animations ? 'smooth' : 'auto', block: 'start' });
  }

  function onScroll(e) {
    const top = e.currentTarget.getBoundingClientRect().top;
    let cur = SECTIONS[0].id;
    for (const sec of SECTIONS) {
      const el = document.getElementById(`sec-${sec.id}`);
      if (el && el.getBoundingClientRect().top - top < 140) cur = sec.id;
    }
    section = cur;
  }
</script>

{#if s}
  <div class="wrap">
    <aside class="toc">
      <h1>Configuración</h1>
      <p class="saved"><Icon name="check" size={13} /> Los cambios se guardan solos</p>
      <nav>
        {#each SECTIONS as sec}
          <button class:sel={section === sec.id} onclick={() => scrollTo(sec.id)}>
            <Icon name={sec.icon} size={16} /> {sec.label}
          </button>
        {/each}
      </nav>
    </aside>

    <div class="content" onscroll={onScroll}>
      <section id="sec-features">
        <h2><Icon name="sparkles" size={18} /> Funciones</h2>
        <div class="card group">
          <SettingRow icon="search" label="Indexadores" description="Buscar en tus indexadores Newznab desde Nebula." checked={s.featureIndexers} onchange={(v) => setFeature('featureIndexers', v)} />
          <SettingRow icon="server" label="Descargas (Usenet / debrid)" description="Descargar, reparar y extraer con tu proveedor Usenet o con un servicio debrid (TorBox, Premiumize). Si la desactivas, desde la búsqueda solo se guardan los .nzb en tu PC." checked={s.featureNntp} onchange={(v) => setFeature('featureNntp', v)} />
        </div>
      </section>

      <section id="sec-debrid" class:dim={!s.featureNntp}>
        <h2><Icon name="cloud" size={18} /> Método de descarga</h2>
        <div class="card group">
          <SettingRow
            icon="download"
            label="Descargar los NZB con"
            description={debridList.length
              ? `NNTP usa tus servidores Usenet. Debrid envía el NZB a ${debridLabel(debridList[0])}${debridList.length > 1 ? ' (o al siguiente si falla)' : ''} y baja los archivos desde sus servidores, normalmente mucho más rápido. Al añadir cada NZB puedes elegir otro método.`
              : 'Añade un servicio debrid (TorBox Pro o Premiumize) para poder descargar a través de él.'}>
            <div class="seg">
              <button class:sel={s.downloadVia !== 'debrid'} onclick={() => saveSettings({ downloadVia: 'nntp' })}><Icon name="server" size={14} /> Usenet (NNTP)</button>
              <button class:sel={s.downloadVia === 'debrid'} disabled={!debridList.length} onclick={() => saveSettings({ downloadVia: 'debrid' })}><Icon name="cloud" size={14} /> Debrid</button>
            </div>
          </SettingRow>
          {#if debridList.length && !hasNntp(s) && s.downloadVia !== 'debrid'}
            <div class="note"><Icon name="info" size={14} /> No tienes servidores NNTP activos: mientras sea así se usará {debridLabel(debridList[0])}.</div>
          {/if}
          <SettingRow label="Si el servicio debrid falla, descargar por NNTP" description="Antes se prueban los demás servicios debrid de la lista (si el método se eligió automáticamente)." checked={s.debridFallbackNntp} onchange={set('debridFallbackNntp')} />
          <SettingRow label="Conexiones al bajar de debrid" description="Descargas simultáneas por HTTPS, por rangos de 32 MB. Más conexiones suelen dar más velocidad.">
            <input class="input small" type="number" min="1" max="32" value={s.debridConnections} onchange={num('debridConnections', 1, 32)} />
          </SettingRow>
          <SettingRow label="Borrar del servicio al terminar" description="Quita la descarga de tu cuenta debrid cuando ya está en tu PC (libera espacio y huecos de descarga)." checked={s.debridDeleteAfter} onchange={set('debridDeleteAfter')} />
          <SettingRow icon="cloud" label="Servicios debrid" description={debridList.length ? `En orden de preferencia: ${debridList.map(debridLabel).join(', ')}.` : 'TorBox (plan Pro) o Premiumize, con su API key.'}>
            <button class="btn sm" onclick={() => (ui.view = 'debrid')}>Gestionar <Icon name="chevron-right" size={14} /></button>
          </SettingRow>
        </div>
      </section>

      <section id="sec-search" class:dim={!s.featureIndexers}>
        <h2><Icon name="search" size={18} /> Búsqueda en indexadores</h2>
        <div class="card group">
          <SettingRow icon="layers" label="Eliminar duplicados" description="Si el mismo release está en varios indexadores (aunque se haya resubido), se muestra una sola vez y se descarga del preferido." checked={s.searchDedup} onchange={set('searchDedup')} />
          <SettingRow icon="zap" label="Descargar con un clic" description="El botón Descargar añade el resultado a la cola con la carpeta y opciones por defecto. Desactivado: se abre la ventana para elegir destino y opciones." checked={s.searchQuickAdd} onchange={set('searchQuickAdd')} disabled={!s.featureNntp} />
          <SettingRow label="Resultados por indexador" description="Por búsqueda; puedes cargar más desde la lista de resultados.">
            <input class="input small" type="number" min="10" max="500" value={s.searchLimit} onchange={num('searchLimit', 10, 500)} />
          </SettingRow>
          <SettingRow label="Carpeta para guardar los .nzb" description="Donde se guardan con el botón «.nzb» (puede ser la carpeta vigilada de otro programa).">
            <input class="input path" value={s.nzbSaveDir} onchange={(e) => saveSettings({ nzbSaveDir: e.target.value })} />
            <button class="btn sm" onclick={() => pick('nzbSaveDir')}>Examinar</button>
          </SettingRow>
          <SettingRow label="Preguntar la carpeta cada vez" checked={s.nzbAskSaveDir} onchange={set('nzbAskSaveDir')} />
          <SettingRow icon="layers" label="Indexadores" description="URL, API key y prioridad de cada indexador.">
            <button class="btn sm" onclick={() => (ui.view = 'indexers')}>Gestionar <Icon name="chevron-right" size={14} /></button>
          </SettingRow>
        </div>
      </section>

      <section id="sec-folders">
        <h2><Icon name="folder" size={18} /> Carpetas</h2>
        <div class="card group">
          <SettingRow label="Carpeta de descargas" description="Destino por defecto de las descargas terminadas.">
            <input class="input path" value={s.downloadDir} onchange={(e) => saveSettings({ downloadDir: e.target.value })} />
            <button class="btn sm" onclick={() => pick('downloadDir')}>Examinar</button>
          </SettingRow>
          <SettingRow label="Carpeta temporal" description="Donde se guardan las descargas en curso. Vacío = subcarpeta «_incompleto» en el destino (más rápido al terminar).">
            <input class="input path" value={s.incompleteDir} placeholder="Automática" onchange={(e) => saveSettings({ incompleteDir: e.target.value })} />
            <button class="btn sm" onclick={() => pick('incompleteDir')}>Examinar</button>
          </SettingRow>
          <SettingRow label="Preguntar dónde guardar cada NZB" description="Muestra la ventana de destino al añadir archivos o enlaces." checked={s.askDestination} onchange={set('askDestination')} />
          <SettingRow label="Crear una subcarpeta por descarga" description="Usa el nombre del NZB como carpeta dentro del destino." checked={s.createJobFolder} onchange={set('createJobFolder')} />
          <SettingRow label="Añadir en pausa" description="Las nuevas descargas no empiezan hasta que las reanudes." checked={s.addPaused} onchange={set('addPaused')} />
        </div>
      </section>

      <section id="sec-post">
        <h2><Icon name="package" size={18} /> Post-procesado</h2>
        <div class="card group">
          <SettingRow icon="shield" label="Verificar y reparar con PAR2" description="Se descargan primero los archivos originales y solo el índice PAR2; los volúmenes de recuperación se piden únicamente si hay daños. Desactivado: no se descarga ningún PAR2." checked={s.par2Enabled} onchange={set('par2Enabled')} />
          <SettingRow icon="archive" label="Extraer archivos comprimidos" description="RAR, 7z, ZIP y volúmenes divididos con el 7-Zip integrado." checked={s.extractEnabled} onchange={set('extractEnabled')} />
          <SettingRow label="Borrar comprimidos tras extraer" description="Elimina los .rar/.7z/.zip cuando la extracción termina bien." checked={s.deleteArchives} onchange={set('deleteArchives')} disabled={!s.extractEnabled} />
          <SettingRow label="Borrar archivos PAR2 al terminar" checked={s.deletePar2} onchange={set('deletePar2')} />
          <SettingRow icon="zap" label="Cancelar descargas imposibles" description="Si faltan más artículos de los que el PAR2 puede reparar, se detiene para no gastar datos." checked={s.abortHopeless} onchange={set('abortHopeless')} />
        </div>
      </section>

      <section id="sec-links">
        <h2><Icon name="clipboard" size={18} /> Enlaces y portapapeles</h2>
        <div class="card group">
          <SettingRow icon="clipboard" label="Vigilar el portapapeles" description="Al copiar un enlace a un .nzb te preguntará si quieres añadirlo. Nunca descarga sin confirmación." checked={s.clipboardMonitor} onchange={set('clipboardMonitor')} />
          <SettingRow label="Analizar el enlace al detectarlo" description="Descarga el NZB para mostrar su nombre y tamaño antes de añadirlo. Algunos indexadores lo cuentan como una descarga (al añadirlo no se vuelve a pedir)." checked={s.clipboardInspect} onchange={set('clipboardInspect')} disabled={!s.clipboardMonitor} />
          <SettingRow label="Mostrar la ventana al detectar un enlace" description="Si Nebula está en la bandeja, se abre automáticamente. Si no, muestra una notificación." checked={s.clipboardShowWindow} onchange={set('clipboardShowWindow')} disabled={!s.clipboardMonitor} />
          <div class="patterns" class:disabled={!s.clipboardMonitor}>
            <div class="p-head">
              <div>
                <div class="p-title">Patrones de enlace</div>
                <div class="p-desc">Además de los enlaces que terminan en <code>.nzb</code>, se detectan los que contienen alguno de estos textos (útil para indexadores).</div>
              </div>
              <button class="btn ghost sm" onclick={resetPatterns}><Icon name="refresh" size={13} /> Restaurar</button>
            </div>
            <div class="chips">
              {#each s.clipboardPatterns as p (p)}
                <span class="pchip" in:scale={{ start: 0.7, duration: d(200) }}>
                  <code>{p}</code>
                  <button onclick={() => saveSettings({ clipboardPatterns: s.clipboardPatterns.filter((x) => x !== p) })} aria-label="Quitar"><Icon name="x" size={12} /></button>
                </span>
              {/each}
              <form class="p-add" onsubmit={(e) => { e.preventDefault(); addPattern(); }}>
                <input class="input" placeholder="Añadir patrón…" bind:value={newPattern} />
              </form>
            </div>
          </div>
        </div>
      </section>

      <section id="sec-windows">
        <h2><Icon name="monitor" size={18} /> Integración con Windows</h2>
        <div class="card group">
          <SettingRow icon="file" label="Abrir archivos .nzb con Nebula" description="Doble clic en un .nzb lo añade a la cola." checked={s.associateNzb} onchange={set('associateNzb')} />
          <SettingRow icon="power" label="Iniciar con Windows" checked={s.startWithWindows} onchange={set('startWithWindows')} />
          <SettingRow label="Iniciar minimizado en la bandeja" checked={s.startMinimized} onchange={set('startMinimized')} />
          <SettingRow label="Minimizar a la bandeja del sistema" description="El botón de minimizar oculta la ventana junto al reloj." checked={s.minimizeToTray} onchange={set('minimizeToTray')} />
          <SettingRow label="Cerrar a la bandeja del sistema" description="Cerrar la ventana no detiene las descargas. Para salir usa el menú del icono de la bandeja." checked={s.closeToTray} onchange={set('closeToTray')} />
          <SettingRow label="Notificar descargas completadas" checked={s.notifyComplete} onchange={set('notifyComplete')} />
          <SettingRow label="Notificar errores" checked={s.notifyFailed} onchange={set('notifyFailed')} />
          <SettingRow label="Confirmar antes de eliminar" checked={s.confirmDelete} onchange={set('confirmDelete')} />
        </div>
      </section>

      <section id="sec-network">
        <h2><Icon name="wifi" size={18} /> Conexión</h2>
        <div class="card group">
          <SettingRow icon="gauge" label="Límite de velocidad" description="En MB/s. 0 = sin límite.">
            <input class="input small" type="number" min="0" step="0.5" value={s.speedLimitKbps ? +(s.speedLimitKbps / 1024).toFixed(1) : 0}
              onchange={(e) => { const v = Math.max(0, parseFloat(String(e.target.value).replace(',', '.')) || 0); saveSettings({ speedLimitKbps: Math.round(v * 1024) }); invoke('set_speed_limit', { kbps: Math.round(v * 1024) }); }} />
            <span class="unit">MB/s</span>
          </SettingRow>
          <SettingRow label="Reintentos por artículo" description="Veces que se reintenta un artículo en el mismo servidor si la conexión falla.">
            <input class="input small" type="number" min="0" max="10" value={s.maxRetries} onchange={num('maxRetries', 0, 10)} />
          </SettingRow>
          <SettingRow label="Tiempo de espera" description="Segundos sin respuesta antes de considerar la conexión caída.">
            <input class="input small" type="number" min="5" max="300" value={s.timeoutSecs} onchange={num('timeoutSecs', 5, 300)} />
            <span class="unit">s</span>
          </SettingRow>
          <SettingRow label="Entradas en el historial" description="Las más antiguas se eliminan de la lista automáticamente.">
            <input class="input small" type="number" min="10" max="5000" value={s.historyLimit} onchange={num('historyLimit', 10, 5000)} />
          </SettingRow>
          <SettingRow icon="server" label="Servidores NNTP" description="Credenciales, conexiones y prioridades.">
            <button class="btn sm" onclick={() => (ui.view = 'servers')}>Gestionar <Icon name="chevron-right" size={14} /></button>
          </SettingRow>
        </div>
      </section>

      <section id="sec-look">
        <h2><Icon name="palette" size={18} /> Apariencia</h2>
        <div class="card group">
          <SettingRow label="Color de acento">
            <div class="swatches">
              {#each ACCENTS as a}
                <button class="swatch" class:sel={s.accent === a.id} style="--a:{a.a}; --b:{a.b}" title={a.name} onclick={() => saveSettings({ accent: a.id })}>
                  {#if s.accent === a.id}<span in:scale={{ duration: d(200) }}><Icon name="check" size={14} stroke={3} /></span>{/if}
                </button>
              {/each}
            </div>
          </SettingRow>
          <SettingRow icon="sparkles" label="Animaciones" description="Desactívalas si prefieres una interfaz más sobria o tu equipo es lento." checked={s.animations} onchange={set('animations')} />
          <SettingRow icon="rocket" label="Asistente de bienvenida" description="Vuelve a mostrar la configuración inicial.">
            <button class="btn sm" onclick={() => (ui.setupOpen = true)}>Abrir</button>
          </SettingRow>
        </div>
      </section>

      <section id="sec-about">
        <h2><Icon name="info" size={18} /> Acerca de</h2>
        <div class="card about">
          <div class="brand">
            <div class="logo-big"><Icon name="download" size={26} stroke={2.4} /></div>
            <div>
              <div class="app-name grad-text">Nebula</div>
              <div class="muted">Versión {ui.info?.version} · Cliente Usenet para Windows</div>
            </div>
          </div>
          <div class="links">
            <button class="btn sm" onclick={() => invoke('open_path', { path: ui.info.dataDir })}><Icon name="folder" size={14} /> Datos y configuración</button>
            <button class="btn sm" onclick={() => invoke('open_path', { path: ui.info.logDir })}><Icon name="logs" size={14} /> Registros</button>
          </div>
          <p class="legal">
            Incluye <strong>7-Zip</strong> © Igor Pavlov (GNU LGPL + restricción unRAR) y <strong>par2cmdline-turbo</strong> (GNU GPL v2),
            ejecutados como programas independientes. Las contraseñas de los servidores se cifran con la protección de datos de Windows (DPAPI).
          </p>
        </div>
      </section>
    </div>
  </div>
{/if}

<style>
  .wrap { display: flex; height: 100%; max-width: 1140px; margin: 0 auto; }
  .toc { width: 230px; flex-shrink: 0; padding: 26px 10px 20px 30px; }
  h1 { margin: 0; font-family: var(--font-display); font-size: 26px; font-weight: 700; }
  .saved { display: flex; align-items: center; gap: 6px; margin: 6px 0 18px; font-size: 12px; color: var(--muted); }
  .toc nav { display: flex; flex-direction: column; gap: 2px; }
  .toc nav button {
    display: flex; align-items: center; gap: 10px;
    height: 36px; padding: 0 12px;
    border: none; border-radius: 9px;
    background: transparent; color: var(--text-2);
    cursor: pointer; text-align: left; font-weight: 550;
    transition: background 0.2s, color 0.2s;
  }
  .toc nav button:hover { background: var(--panel-2); color: var(--text); }
  .toc nav button.sel { background: rgba(var(--accent-rgb), 0.14); color: var(--text); }
  .toc nav button.sel :global(svg) { color: var(--accent-2); }
  .content { flex: 1; overflow-y: auto; padding: 26px 30px 120px 20px; scroll-behavior: smooth; }
  section { margin-bottom: 30px; scroll-margin-top: 20px; transition: opacity 0.25s; }
  section.dim { opacity: 0.5; }
  h2 { display: flex; align-items: center; gap: 10px; font-size: 15px; font-weight: 650; margin: 0 0 12px; color: var(--text); }
  h2 :global(svg) { color: var(--accent-2); }
  .group { overflow: hidden; }
  .path { width: 280px; height: 32px; font-size: 12.5px; }
  .small { width: 90px; height: 32px; text-align: right; }
  .unit { font-size: 12.5px; color: var(--muted); width: 32px; }
  .patterns { padding: 14px 18px 16px; border-top: 1px solid var(--border); transition: opacity 0.2s; }
  .patterns.disabled { opacity: 0.5; pointer-events: none; }
  .p-head { display: flex; justify-content: space-between; gap: 14px; }
  .p-title { font-weight: 560; }
  .p-desc { font-size: 12.5px; color: var(--muted); margin-top: 2px; }
  code { font-family: var(--mono); font-size: 12px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px; align-items: center; }
  .pchip { display: inline-flex; align-items: center; gap: 4px; height: 28px; padding: 0 4px 0 10px; border-radius: 8px; background: var(--panel-3); border: 1px solid var(--border-2); }
  .pchip button { display: grid; place-items: center; width: 20px; height: 20px; border: none; border-radius: 5px; background: none; color: var(--muted); cursor: pointer; }
  .pchip button:hover { background: rgba(248, 113, 113, 0.18); color: var(--err); }
  .p-add .input { height: 28px; width: 160px; font-size: 12.5px; }
  .seg { display: flex; padding: 3px; gap: 3px; border-radius: 10px; background: var(--panel-2); border: 1px solid var(--border); }
  .seg button {
    display: inline-flex; align-items: center; gap: 6px;
    height: 28px; padding: 0 12px; border: none; border-radius: 7px;
    background: transparent; color: var(--text-2); font-size: 12.5px; font-weight: 600; cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .seg button:hover:not(:disabled) { color: var(--text); }
  .seg button.sel { background: rgba(var(--accent-rgb), 0.2); color: var(--text); box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.45); }
  .seg button:disabled { opacity: 0.4; cursor: not-allowed; }
  .note { display: flex; align-items: center; gap: 8px; padding: 0 18px 12px 64px; margin-top: -4px; font-size: 12.5px; color: var(--muted); }
  .swatches { display: flex; gap: 10px; }
  .swatch {
    width: 30px; height: 30px; border-radius: 50%;
    border: 2px solid transparent;
    background: linear-gradient(135deg, var(--a), var(--b));
    cursor: pointer; display: grid; place-items: center; color: #fff;
    transition: transform 0.2s var(--ease), box-shadow 0.2s;
  }
  .swatch:hover { transform: scale(1.12); }
  .swatch.sel { box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--a); }
  .about { padding: 20px; }
  .brand { display: flex; align-items: center; gap: 16px; }
  .logo-big { width: 56px; height: 56px; border-radius: 16px; display: grid; place-items: center; background: var(--grad-diag); color: #fff; box-shadow: 0 10px 30px -10px rgba(var(--accent-rgb), 0.9); }
  .app-name { font-family: var(--font-display); font-size: 22px; font-weight: 750; }
  .links { display: flex; gap: 8px; margin-top: 16px; }
  .legal { font-size: 12px; color: var(--muted); line-height: 1.6; margin: 16px 0 0; }
</style>
