<script>
  import { invoke } from '@tauri-apps/api/core';
  import { fly, fade } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import ServerForm from '../components/ServerForm.svelte';
  import Switch from '../components/Switch.svelte';
  import { ui, saveSettings, ask, toast, d } from '../lib/state.svelte.js';
  import * as f from '../lib/format.js';

  let editing = $state(null);
  let isNew = $state(false);
  let testing = $state({});

  const servers = $derived([...(ui.settings?.servers ?? [])].sort((a, b) => a.priority - b.priority));
  const minPrio = $derived(Math.min(...servers.filter((s) => s.enabled).map((s) => s.priority), 99));

  function runtime(id) {
    return ui.queue.servers.find((s) => s.id === id);
  }

  function blank() {
    return {
      id: crypto.randomUUID().replaceAll('-', ''),
      name: '',
      host: '',
      port: 563,
      ssl: true,
      verifyCert: true,
      username: '',
      password: '',
      passwordEnc: '',
      connections: 10,
      priority: servers.length ? Math.max(...servers.map((s) => s.priority)) + 1 : 0,
      enabled: true,
      retentionDays: 0,
    };
  }

  function add() {
    editing = blank();
    isNew = true;
  }

  function edit(s) {
    editing = structuredClone($state.snapshot(s));
    isNew = false;
  }

  async function save() {
    if (!editing.host.trim()) {
      toast('error', 'Indica la dirección del servidor.');
      return;
    }
    editing.port = Number(editing.port) || (editing.ssl ? 563 : 119);
    editing.priority = Number(editing.priority) || 0;
    editing.retentionDays = Number(editing.retentionDays) || 0;
    const list = $state.snapshot(ui.settings.servers);
    const i = list.findIndex((s) => s.id === editing.id);
    const item = $state.snapshot(editing);
    if (i >= 0) list[i] = item;
    else list.push(item);
    if (await saveSettings({ servers: list })) {
      toast('success', isNew ? 'Servidor añadido' : 'Servidor actualizado', 2500);
      editing = null;
    }
  }

  async function remove(s) {
    const r = await ask({ title: '¿Eliminar servidor?', message: `${s.name || s.host} dejará de usarse para descargar.`, confirmLabel: 'Eliminar', danger: true, icon: 'trash' });
    if (!r.ok) return;
    await saveSettings({ servers: $state.snapshot(ui.settings.servers).filter((x) => x.id !== s.id) });
  }

  async function toggle(s, v) {
    const list = $state.snapshot(ui.settings.servers).map((x) => (x.id === s.id ? { ...x, enabled: v } : x));
    await saveSettings({ servers: list });
  }

  async function shift(s, delta) {
    const list = $state.snapshot(ui.settings.servers).map((x) => (x.id === s.id ? { ...x, priority: Math.max(0, x.priority + delta) } : x));
    await saveSettings({ servers: list });
  }

  async function test(s) {
    testing[s.id] = { busy: true };
    try {
      const r = await invoke('test_server', { server: $state.snapshot(s) });
      testing[s.id] = r;
    } catch (e) {
      testing[s.id] = { ok: false, message: String(e) };
    }
  }

  const STATE = {
    active: { cls: 'active', text: 'Descargando' },
    idle: { cls: 'idle', text: 'Conectado' },
    ready: { cls: 'ok', text: 'Listo' },
    warn: { cls: 'warn', text: 'Con errores' },
    down: { cls: 'err', text: 'Sin conexión' },
    auth: { cls: 'err', text: 'Credenciales rechazadas' },
  };
</script>

<div class="view">
  <header class="head">
    <div>
      <h1>Servidores</h1>
      <p class="subtitle">Tus proveedores Usenet. Los de respaldo solo se usan cuando al principal le falta un artículo.</p>
    </div>
    <button class="btn primary" onclick={add}><Icon name="plus" size={16} /> Añadir servidor</button>
  </header>

  <div class="list">
    {#each servers as s (s.id)}
      {@const rt = runtime(s.id)}
      {@const stt = s.enabled ? STATE[rt?.state ?? 'ready'] : { cls: '', text: 'Desactivado' }}
      {@const t = testing[s.id]}
      <article class="srv card" class:off={!s.enabled} animate:flip={{ duration: d(350), easing: cubicOut }} in:fly={{ y: 14, duration: d(300) }} out:fade={{ duration: d(180) }}>
        <div class="srv-top">
          <div class="srv-icon" class:primary={s.priority === minPrio && s.enabled}><Icon name="server" size={20} /></div>
          <div class="srv-main">
            <div class="srv-name">
              {s.name || s.host}
              <span class="chip {s.priority === minPrio && s.enabled ? 'accent' : 'muted'}">
                {s.priority === minPrio ? 'Principal' : `Respaldo · prioridad ${s.priority}`}
              </span>
              {#if s.ssl}<span class="chip ok"><Icon name="lock" size={10} stroke={2.6} /> SSL</span>{/if}
            </div>
            <div class="srv-sub">{s.host}:{s.port} · {s.username || 'sin usuario'} · {s.connections} conexiones{#if s.retentionDays} · {s.retentionDays} días de retención{/if}</div>
          </div>
          <div class="srv-actions">
            <button class="icon-btn" title="Subir prioridad" onclick={() => shift(s, -1)} disabled={s.priority === 0}><Icon name="chevron-up" size={17} /></button>
            <button class="icon-btn" title="Bajar prioridad" onclick={() => shift(s, 1)}><Icon name="chevron-down" size={17} /></button>
            <button class="icon-btn" title="Probar conexión" onclick={() => test(s)}><Icon name="plug" size={16} /></button>
            <button class="icon-btn" title="Editar" onclick={() => edit(s)}><Icon name="edit" size={16} /></button>
            <button class="icon-btn danger" title="Eliminar" onclick={() => remove(s)}><Icon name="trash" size={16} /></button>
            <Switch checked={s.enabled} onchange={(v) => toggle(s, v)} label="Activo" />
          </div>
        </div>
        <div class="srv-stats">
          <div class="st"><span class="dot {stt.cls}"></span>{stt.text}</div>
          {#if rt}
            <div class="st"><Icon name="plug" size={13} /> {rt.active}/{rt.maxConns} activas · {rt.connected} abiertas</div>
            <div class="st"><Icon name="download" size={13} /> {f.bytes(rt.bytes)} en esta sesión</div>
            <div class="st"><Icon name="check" size={13} /> {rt.articlesOk.toLocaleString('es-ES')} artículos</div>
            {#if rt.articlesMissing}<div class="st warn"><Icon name="alert-triangle" size={13} /> {rt.articlesMissing.toLocaleString('es-ES')} no encontrados</div>{/if}
          {/if}
        </div>
        {#if rt?.lastError && s.enabled}
          <div class="alert err"><Icon name="x-circle" size={15} /><span class="selectable">{rt.lastError}</span></div>
        {/if}
        {#if t}
          <div class="alert {t.busy ? 'info' : t.ok ? 'ok' : 'err'}" in:fly={{ y: -6, duration: d(200) }}>
            <Icon name={t.busy ? 'loader' : t.ok ? 'check-circle' : 'x-circle'} size={15} class={t.busy ? 'spin' : ''} />
            <span class="selectable">{t.busy ? 'Probando conexión…' : `${t.message}${t.ok ? ` · ${t.latencyMs} ms` : ''}`}</span>
          </div>
        {/if}
      </article>
    {:else}
      <div class="empty card" in:fade>
        <Icon name="server" size={36} stroke={1.6} />
        <h3>Aún no hay servidores</h3>
        <p>Añade las credenciales NNTP de tu proveedor Usenet para empezar a descargar.</p>
        <button class="btn primary" onclick={add}><Icon name="plus" size={16} /> Añadir servidor</button>
      </div>
    {/each}
  </div>

  {#if servers.length > 1}
    <div class="alert info tip">
      <Icon name="info" size={16} />
      <span>Los servidores con la misma prioridad se reparten el trabajo. Si un artículo no está en el principal, se busca en los de respaldo por orden.</span>
    </div>
  {/if}
</div>

{#if editing}
  <Modal title={isNew ? 'Nuevo servidor' : 'Editar servidor'} subtitle="Credenciales NNTP de tu proveedor" icon="server" width={640} onclose={() => (editing = null)}>
    <ServerForm bind:server={editing} />
    {#snippet footer()}
      <button class="btn ghost" onclick={() => (editing = null)}>Cancelar</button>
      <button class="btn primary" onclick={save}><Icon name="check" size={16} /> Guardar</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .view { padding: 26px 30px 40px; max-width: 1100px; margin: 0 auto; }
  .head { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; }
  h1 { margin: 0; font-family: var(--font-display); font-size: 26px; font-weight: 700; }
  .subtitle { margin: 4px 0 0; color: var(--text-2); }
  .list { display: flex; flex-direction: column; gap: 12px; margin-top: 22px; }
  .srv { padding: 16px 18px; display: flex; flex-direction: column; gap: 12px; transition: opacity 0.3s, border-color 0.2s; }
  .srv.off { opacity: 0.6; }
  .srv-top { display: flex; align-items: center; gap: 14px; }
  .srv-icon { width: 44px; height: 44px; border-radius: 12px; display: grid; place-items: center; background: var(--panel-3); color: var(--text-2); flex-shrink: 0; }
  .srv-icon.primary { background: var(--grad-diag); color: #fff; box-shadow: 0 8px 22px -10px rgba(var(--accent-rgb), 0.9); }
  .srv-main { flex: 1; min-width: 0; }
  .srv-name { display: flex; align-items: center; gap: 8px; font-weight: 620; font-size: 15px; flex-wrap: wrap; }
  .srv-sub { color: var(--text-2); font-size: 12.5px; margin-top: 3px; }
  .srv-actions { display: flex; align-items: center; gap: 2px; }
  .srv-actions :global(.switch) { margin-left: 8px; }
  .srv-stats { display: flex; flex-wrap: wrap; gap: 8px 20px; padding-top: 12px; border-top: 1px solid var(--border); }
  .st { display: flex; align-items: center; gap: 7px; font-size: 12.5px; color: var(--text-2); }
  .st.warn { color: #fcd34d; }
  .empty { padding: 48px 20px; text-align: center; color: var(--text-2); display: flex; flex-direction: column; align-items: center; gap: 8px; }
  .empty h3 { margin: 8px 0 0; color: var(--text); font-family: var(--font-display); }
  .empty p { margin: 0 0 12px; }
  .tip { margin-top: 16px; }
</style>
