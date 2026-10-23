<script>
  import Switch from './Switch.svelte';
  import Icon from './Icon.svelte';

  let { label, description = '', checked = undefined, onchange = null, disabled = false, icon = null, children } = $props();
  const isToggle = $derived(checked !== undefined);
</script>

<div
  class="srow"
  class:clickable={isToggle && !disabled}
  class:disabled
  class:custom={!isToggle}
  onclick={() => isToggle && !disabled && onchange?.(!checked)}
  role="presentation">
  {#if icon}
    <div class="s-icon"><Icon name={icon} size={17} /></div>
  {/if}
  <div class="text">
    <div class="s-label">{label}</div>
    {#if description}<div class="s-desc">{description}</div>{/if}
  </div>
  <div class="control">
    {#if isToggle}
      <Switch {checked} {onchange} {disabled} {label} />
    {:else}
      {@render children?.()}
    {/if}
  </div>
</div>

<style>
  .srow {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 13px 18px;
    border-top: 1px solid var(--border);
    transition: background 0.15s;
  }
  .srow:first-child { border-top: none; }
  .srow.clickable { cursor: pointer; }
  .srow.clickable:hover { background: rgba(255, 255, 255, 0.02); }
  .srow.disabled { opacity: 0.5; }
  .s-icon {
    width: 32px; height: 32px;
    border-radius: 9px;
    display: grid; place-items: center;
    background: var(--panel-3);
    color: var(--text-2);
    flex-shrink: 0;
  }
  .text { flex: 1; min-width: 0; }
  .s-label { font-weight: 560; }
  .s-desc { font-size: 12.5px; color: var(--muted); margin-top: 2px; line-height: 1.4; }
  .control { flex-shrink: 0; display: flex; align-items: center; gap: 8px; }
  /* Móvil: los controles que no son interruptores (campos, botones) bajan bajo el texto. */
  @media (max-width: 720px) {
    .srow { padding: 12px 14px; gap: 12px; }
    .srow.custom { flex-wrap: wrap; }
    .srow.custom .control { width: 100%; justify-content: flex-end; }
    .srow.custom .control :global(.input:not(.small)) { flex: 1; min-width: 0; }
    .srow.custom .control :global(.select) { flex: 1; }
  }
</style>
