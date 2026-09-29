<script>
  import { invoke } from '@tauri-apps/api/core';
  import { fly } from 'svelte/transition';
  import Icon from './Icon.svelte';
  import Switch from './Switch.svelte';
  import { d } from '../lib/state.svelte.js';

  let { server = $bindable(), compact = false } = $props();
  let showPw = $state(false);
  let testing = $state(false);
  let result = $state(null);

  function toggleSsl(v) {
    server.ssl = v;
    if (v && server.port === 119) server.port = 563;
    if (!v && server.port === 563) server.port = 119;
  }

  export async function test() {
    if (!server.host.trim()) {
      result = { ok: false, message: 'Indica la dirección del servidor.' };
      return false;
    }
    testing = true;
    result = null;
    try {
      result = await invoke('test_server', { server: $state.snapshot(server) });
    } catch (e) {
      result = { ok: false, message: String(e) };
    } finally {
      testing = false;
    }
    return result.ok;
  }
</script>

<div class="sf" class:compact>
  <div class="grid">
    <div class="field span2">
      <label for="sf-host">Servidor (host)</label>
      <input id="sf-host" class="input" placeholder="news.proveedor.com" bind:value={server.host} spellcheck="false" />
    </div>
    <div class="field">
      <label for="sf-port">Puerto</label>
      <input id="sf-port" class="input" type="number" min="1" max="65535" bind:value={server.port} />
    </div>
    <div class="field">
      <label for="sf-user">Usuario</label>
      <input id="sf-user" class="input" bind:value={server.username} autocomplete="off" spellcheck="false" />
    </div>
    <div class="field span2">
      <label for="sf-pass">Contraseña</label>
      <div class="pw">
        <input id="sf-pass" class="input" type={showPw ? 'text' : 'password'} bind:value={server.password} autocomplete="new-password" />
        <button class="icon-btn eye" onclick={() => (showPw = !showPw)} aria-label="Mostrar contraseña"><Icon name={showPw ? 'eye-off' : 'eye'} size={16} /></button>
      </div>
    </div>
    <div class="field span2">
      <label for="sf-conns">Conexiones simultáneas <span class="val">{server.connections}</span></label>
      <input id="sf-conns" class="range" type="range" min="1" max="100" bind:value={server.connections} style="--p:{(server.connections - 1) / 0.99}%" />
      <span class="hint">Consulta el máximo que permite tu plan. Más conexiones no siempre es más rápido.</span>
    </div>
    <div class="field">
      <label for="sf-name">Nombre (opcional)</label>
      <input id="sf-name" class="input" placeholder={server.host || 'Mi servidor'} bind:value={server.name} />
    </div>
    {#if !compact}
      <div class="field">
        <label for="sf-prio">Prioridad</label>
        <select id="sf-prio" class="select" bind:value={server.priority}>
          <option value={0}>Principal</option>
          {#each [1, 2, 3, 4, 5] as p}<option value={p}>Respaldo {p}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label for="sf-ret">Retención (días)</label>
        <input id="sf-ret" class="input" type="number" min="0" bind:value={server.retentionDays} placeholder="0 = desconocida" />
      </div>
    {/if}
  </div>

  <div class="toggles">
    <label class="opt"><Switch checked={server.ssl} onchange={toggleSsl} /> <Icon name="lock" size={14} /> Conexión cifrada (SSL/TLS)</label>
    {#if server.ssl}
      <label class="opt"><Switch checked={server.verifyCert} onchange={(v) => (server.verifyCert = v)} /> Verificar certificado</label>
    {/if}
    {#if !compact}
      <label class="opt"><Switch checked={server.enabled} onchange={(v) => (server.enabled = v)} /> Servidor activo</label>
    {/if}
  </div>

  <div class="test-row">
    <button class="btn" onclick={test} disabled={testing}>
      {#if testing}<Icon name="loader" size={15} class="spin" /> Probando…{:else}<Icon name="plug" size={15} /> Probar conexión{/if}
    </button>
    {#if result}
      <div class="result {result.ok ? 'ok' : 'err'}" in:fly={{ x: -10, duration: d(250) }}>
        <Icon name={result.ok ? 'check-circle' : 'x-circle'} size={16} />
        <span class="selectable">{result.message}{#if result.ok && result.latencyMs} · {result.latencyMs} ms{/if}</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .sf { display: flex; flex-direction: column; gap: 16px; }
  .grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 14px; }
  .span2 { grid-column: span 2; }
  .pw { position: relative; }
  .pw .input { padding-right: 40px; }
  .eye { position: absolute; right: 3px; top: 3px; }
  .val { float: right; color: var(--text); font-variant-numeric: tabular-nums; }
  .range {
    -webkit-appearance: none; appearance: none;
    width: 100%; height: 6px; margin: 10px 0 4px;
    border-radius: 99px; outline: none;
    background: linear-gradient(90deg, var(--accent), var(--accent-2)) 0 / var(--p) 100% no-repeat, var(--panel-4);
  }
  .range::-webkit-slider-thumb {
    -webkit-appearance: none; appearance: none;
    width: 18px; height: 18px; border-radius: 50%;
    background: #fff; cursor: pointer;
    box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.3), 0 2px 6px rgba(0, 0, 0, 0.4);
    transition: box-shadow 0.2s;
  }
  .range::-webkit-slider-thumb:hover { box-shadow: 0 0 0 6px rgba(var(--accent-rgb), 0.3), 0 2px 6px rgba(0, 0, 0, 0.4); }
  .toggles { display: flex; flex-wrap: wrap; gap: 12px 24px; }
  .opt { display: flex; align-items: center; gap: 9px; font-size: 13px; color: var(--text-2); cursor: pointer; }
  .test-row { display: flex; align-items: center; gap: 14px; min-height: 36px; }
  .result { display: flex; align-items: center; gap: 8px; font-size: 13px; min-width: 0; }
  .result.ok { color: #6ee7b7; }
  .result.err { color: #fca5a5; }
</style>
