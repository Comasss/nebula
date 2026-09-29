<script>
  let { data = [], height = 60, strokeWidth = 2, id = 'g' } = $props();
  const W = 300;

  const path = $derived.by(() => {
    const n = data.length;
    if (n < 2) return { line: '', area: '' };
    const max = Math.max(...data, 1) * 1.18;
    const pts = data.map((v, i) => [(i / (n - 1)) * W, height - (v / max) * (height - 4) - 2]);
    let line = `M${pts[0][0].toFixed(1)},${pts[0][1].toFixed(1)}`;
    for (let i = 1; i < n; i++) {
      const [x0, y0] = pts[i - 1];
      const [x1, y1] = pts[i];
      const cx = (x0 + x1) / 2;
      line += ` C${cx.toFixed(1)},${y0.toFixed(1)} ${cx.toFixed(1)},${y1.toFixed(1)} ${x1.toFixed(1)},${y1.toFixed(1)}`;
    }
    const area = `${line} L${W},${height} L0,${height} Z`;
    return { line, area };
  });
</script>

<svg class="graph" viewBox="0 0 {W} {height}" preserveAspectRatio="none" style="height:{height}px">
  <defs>
    <linearGradient id="{id}-stroke" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" style="stop-color: var(--accent)" />
      <stop offset="1" style="stop-color: var(--accent-2)" />
    </linearGradient>
    <linearGradient id="{id}-fill" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" style="stop-color: var(--accent); stop-opacity: 0.32" />
      <stop offset="1" style="stop-color: var(--accent); stop-opacity: 0" />
    </linearGradient>
  </defs>
  <path class="area" d={path.area} fill="url(#{id}-fill)" />
  <path class="line" d={path.line} fill="none" stroke="url(#{id}-stroke)" stroke-width={strokeWidth} vector-effect="non-scaling-stroke" stroke-linecap="round" />
</svg>

<style>
  .graph { width: 100%; display: block; overflow: visible; }
  .area, .line { transition: d 0.48s linear; }
</style>
