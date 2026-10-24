<script>
  import { haptic } from '../lib/feedback.js';
  let { checked = false, disabled = false, onchange = null, label = '' } = $props();
</script>

<button
  type="button"
  class="switch"
  class:on={checked}
  {disabled}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  onclick={(e) => {
    e.stopPropagation();
    haptic('tick');
    onchange?.(!checked);
  }}>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    width: 40px;
    height: 23px;
    flex-shrink: 0;
    border-radius: 999px;
    border: 1px solid var(--border-2);
    background: var(--panel-4);
    cursor: pointer;
    padding: 0;
    transition: background 0.25s var(--ease), border-color 0.25s;
  }
  .switch.on {
    background: var(--grad-diag);
    border-color: transparent;
    box-shadow: 0 0 14px -4px rgba(var(--accent-rgb), 0.9);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    transition: transform 0.28s var(--ease), width 0.2s var(--ease);
  }
  .switch.on .knob { transform: translateX(17px); }
  .switch:active .knob { width: 21px; }
  .switch.on:active .knob { transform: translateX(13px); }
  .switch:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
