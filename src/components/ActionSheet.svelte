<script>
  // Hoja de acciones del móvil: sale desde abajo al mantener pulsada una descarga o al tocar ⋯.
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte.js';
  import { ui, d } from '../lib/state.svelte.js';
  import { haptic } from '../lib/feedback.js';

  const m = $derived(ui.actionSheet);

  function pick(a) {
    if (a.disabled) return;
    haptic(a.danger ? 'long' : 'tap');
    m.resolve(a.id);
  }
  function onkey(e) {
    if (m && e.key === 'Escape') {
      e.stopPropagation();
      m.resolve(null);
    }
  }
</script>

<svelte:window onkeydown={onkey} />

{#if m}
  <div class="backdrop" transition:fade={{ duration: d(160) }} onclick={() => m.resolve(null)} role="presentation"></div>
  <div class="sheet" role="dialog" transition:fly={{ y: 320, duration: d(320), easing: cubicOut }}>
    <div class="panel glass">
      <div class="grip"></div>
      {#if m.title}
        <div class="head">
          <strong class="clamp">{m.title}</strong>
          {#if m.subtitle}<span class="truncate">{m.subtitle}</span>{/if}
        </div>
      {/if}
      <div class="items">
        {#each m.actions as a, i (a.id)}
          <button type="button" class="act" class:danger={a.danger} disabled={a.disabled} style="--i:{i}" onclick={() => pick(a)}>
            <span class="a-ic"><Icon name={a.icon} size={17} /></span>
            <span>{a.label}</span>
          </button>
        {/each}
      </div>
    </div>
    <button type="button" class="cancel glass" onclick={() => m.resolve(null)}>{t('common.cancel')}</button>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 300; background: rgba(5, 6, 10, 0.5); backdrop-filter: blur(3px); }
  .sheet {
    position: fixed; left: 10px; right: 10px;
    bottom: calc(10px + env(safe-area-inset-bottom, 0px));
    z-index: 301;
    display: flex; flex-direction: column; gap: 8px;
  }
  .panel { border-radius: 26px; padding: 8px 8px 8px; max-height: 70vh; overflow-y: auto; }
  .grip { width: 40px; height: 4px; border-radius: 4px; background: rgba(255, 255, 255, 0.22); margin: 2px auto 8px; }
  .head { display: flex; flex-direction: column; gap: 2px; padding: 2px 12px 10px; border-bottom: 1px solid rgba(255, 255, 255, 0.07); margin-bottom: 4px; }
  .head strong { font-size: 14px; word-break: break-all; }
  .head span { font-size: 12px; color: var(--muted); }
  .clamp { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .items { display: flex; flex-direction: column; }
  .act {
    display: flex; align-items: center; gap: 14px;
    width: 100%; min-height: 52px; padding: 0 12px;
    border: none; border-radius: 14px; background: transparent;
    color: var(--text); font-size: 15px; font-weight: 560; text-align: left; cursor: pointer;
    transition: background 0.15s, transform 0.15s var(--ease);
    animation: rise 0.32s var(--ease) backwards;
    animation-delay: calc(var(--i) * 30ms);
  }
  .act:active { background: rgba(255, 255, 255, 0.07); transform: scale(0.98); }
  .act:disabled { opacity: 0.35; }
  .a-ic { width: 32px; height: 32px; border-radius: 10px; display: grid; place-items: center; background: rgba(255, 255, 255, 0.06); color: var(--accent-2); }
  .act.danger { color: #fca5a5; }
  .act.danger .a-ic { background: rgba(248, 113, 113, 0.14); color: var(--err); }
  @keyframes rise { from { opacity: 0; transform: translateY(8px); } }
  .cancel { height: 54px; border-radius: 20px; font-size: 15.5px; font-weight: 650; color: var(--text); cursor: pointer; }
  .cancel:active { transform: scale(0.98); }
</style>
