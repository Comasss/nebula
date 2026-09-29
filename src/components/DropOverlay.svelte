<script>
  import { t } from '../lib/i18n.svelte.js';
  import { fade, scale } from 'svelte/transition';
  import Icon from './Icon.svelte';
  import { ui, d } from '../lib/state.svelte.js';
</script>

{#if ui.dragging}
  <div class="drop" transition:fade={{ duration: d(150) }}>
    <div class="zone" in:scale={{ start: 0.92, duration: d(260) }}>
      <div class="d-icon"><Icon name="upload" size={40} stroke={1.8} /></div>
      <div class="d-title">{t('drop.title')}</div>
      <div class="d-sub">{t('drop.subtitle')}</div>
    </div>
  </div>
{/if}

<style>
  .drop {
    position: fixed;
    inset: 40px 0 0 0;
    z-index: 400;
    display: grid;
    place-items: center;
    background: rgba(6, 7, 12, 0.78);
    backdrop-filter: blur(8px);
    pointer-events: none;
  }
  .zone {
    width: min(560px, 80vw);
    padding: 56px 24px;
    border-radius: 24px;
    border: 2px dashed rgba(var(--accent-rgb), 0.7);
    background: radial-gradient(circle at 50% 30%, rgba(var(--accent-rgb), 0.18), transparent 70%);
    text-align: center;
    animation: breathe 1.8s ease-in-out infinite;
  }
  @keyframes breathe {
    50% { border-color: rgba(var(--accent-rgb), 1); box-shadow: 0 0 60px -10px rgba(var(--accent-rgb), 0.6); }
  }
  .d-icon {
    width: 84px; height: 84px;
    margin: 0 auto 18px;
    border-radius: 24px;
    display: grid; place-items: center;
    background: var(--grad-diag);
    color: #fff;
    animation: bob 1.6s ease-in-out infinite;
  }
  @keyframes bob { 50% { transform: translateY(-8px); } }
  .d-title { font-family: var(--font-display); font-size: 22px; font-weight: 650; }
  .d-sub { color: var(--text-2); margin-top: 6px; }
</style>
