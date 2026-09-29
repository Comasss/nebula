<script>
  import { fly, fade } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { ui, dismissToast, d } from '../lib/state.svelte.js';

  const ICON = { success: 'check-circle', error: 'x-circle', warn: 'alert-triangle', info: 'info' };
</script>

<div class="toasts">
  {#each ui.toasts as t (t.id)}
    <div
      class="toast {t.kind}"
      animate:flip={{ duration: d(300) }}
      in:fly={{ x: 40, duration: d(320), easing: cubicOut }}
      out:fade={{ duration: d(180) }}>
      <Icon name={ICON[t.kind] ?? 'info'} size={18} />
      <div class="msg selectable">{t.message}</div>
      <button class="icon-btn close" onclick={() => dismissToast(t.id)} aria-label="Cerrar"><Icon name="x" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 20px;
    bottom: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    z-index: 300;
    width: 380px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 10px 12px 14px;
    border-radius: 12px;
    background: rgba(24, 27, 37, 0.96);
    border: 1px solid var(--border-2);
    box-shadow: var(--shadow);
    backdrop-filter: blur(10px);
    font-size: 13px;
    line-height: 1.4;
  }
  .toast :global(svg) { margin-top: 1px; }
  .toast.success :global(svg:first-child) { color: var(--ok); }
  .toast.error { border-color: rgba(248, 113, 113, 0.35); }
  .toast.error :global(svg:first-child) { color: var(--err); }
  .toast.warn :global(svg:first-child) { color: var(--warn); }
  .toast.info :global(svg:first-child) { color: var(--info); }
  .msg { flex: 1; min-width: 0; word-break: break-word; }
  .close { width: 24px; height: 24px; margin: -3px 0; }
</style>
