<script>
  let { value = 0, kind = 'accent', active = false, height = 8 } = $props();
  const pct = $derived(Math.max(0, Math.min(100, value || 0)));
</script>

<div class="pbar {kind}" class:active style="height:{height}px" role="progressbar" aria-valuenow={pct} aria-valuemin="0" aria-valuemax="100">
  <div class="fill" style="width:{pct}%"></div>
</div>

<style>
  .pbar {
    position: relative;
    width: 100%;
    background: var(--panel-3);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill {
    position: relative;
    height: 100%;
    min-width: 0;
    border-radius: inherit;
    background: var(--grad);
    transition: width 0.7s var(--ease), background 0.4s;
    overflow: hidden;
  }
  .active .fill { box-shadow: 0 0 16px -2px rgba(var(--accent-rgb), 0.7); }
  .active .fill::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent 0%, rgba(255, 255, 255, 0.32) 50%, transparent 100%);
    transform: translateX(-100%);
    animation: shimmer 1.8s ease-in-out infinite;
  }
  .paused .fill, .muted .fill { background: #454b5e; box-shadow: none; }
  .info .fill { background: linear-gradient(90deg, #3b82f6, #60a5fa); box-shadow: 0 0 14px -3px rgba(96, 165, 250, 0.7); }
  .warn .fill { background: linear-gradient(90deg, #f59e0b, #fbbf24); box-shadow: 0 0 14px -3px rgba(251, 191, 36, 0.7); }
  .teal .fill { background: linear-gradient(90deg, #14b8a6, #34d399); box-shadow: 0 0 14px -3px rgba(45, 212, 191, 0.7); }
  .ok .fill { background: linear-gradient(90deg, #10b981, #34d399); }
  .err .fill { background: linear-gradient(90deg, #ef4444, #f87171); }
  @keyframes shimmer {
    to { transform: translateX(100%); }
  }
</style>
