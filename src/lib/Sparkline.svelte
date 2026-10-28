<script lang="ts">
  let {
    values,
    color = '#4f6ef7',
    height = 24,
    showLabels = true,
  }: {
    values: number[];
    color?: string;
    height?: number;
    /** Hide min/max labels for very narrow contexts (default: show) */
    showLabels?: boolean;
  } = $props();

  const hasData = $derived(values.length >= 2);
  const min = $derived(hasData ? Math.min(...values) : 0);
  const max = $derived(hasData ? Math.max(...values) : 1);
  const range = $derived(max - min || 1);

  const polyline = $derived(
    hasData
      ? values
          .map((v, i) => {
            const x = (i / (values.length - 1)) * 100;
            const y = 100 - ((v - min) / range) * 80 - 10;
            return `${x.toFixed(2)},${y.toFixed(2)}`;
          })
          .join(' ')
      : ''
  );

  const areaPath = $derived(
    hasData ? `M 0,100 L ${polyline.replaceAll(' ', ' L ')} L 100,100 Z` : ''
  );

  /** Pick a readable format given the magnitude of the value */
  function fmt(n: number): string {
    if (!Number.isFinite(n)) return '—';
    const abs = Math.abs(n);
    if (abs === 0) return '0';
    if (abs >= 10000) return n.toExponential(1);
    if (Number.isInteger(n)) return n.toString();
    if (abs >= 100) return n.toFixed(1);
    if (abs >= 1) return n.toFixed(2);
    if (abs >= 0.01) return n.toFixed(3);
    return n.toExponential(1);
  }
</script>

<div class="spark-wrap" style="height: {height}px">
  {#if showLabels && hasData}
    <span class="spark-label min" style="color: {color}88">{fmt(min)}</span>
  {/if}
  <svg
    viewBox="0 0 100 100"
    preserveAspectRatio="none"
    class="spark"
  >
    {#if hasData}
      <defs>
        <linearGradient id="spark-grad-{color.replace('#', '')}" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color={color} stop-opacity="0.3" />
          <stop offset="100%" stop-color={color} stop-opacity="0" />
        </linearGradient>
      </defs>
      <path d={areaPath} fill="url(#spark-grad-{color.replace('#', '')})" />
      <polyline
        points={polyline}
        fill="none"
        stroke={color}
        stroke-width="1.5"
        stroke-linejoin="round"
        vector-effect="non-scaling-stroke"
      />
    {:else}
      <line x1="0" y1="50" x2="100" y2="50" stroke="#2a2a42" stroke-width="0.5" stroke-dasharray="2,2" />
    {/if}
  </svg>
  {#if showLabels && hasData}
    <span class="spark-label max" style="color: {color}88">{fmt(max)}</span>
  {/if}
</div>

<style>
  .spark-wrap {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    width: 100%;
  }

  .spark {
    display: block;
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  .spark-label {
    font-family: "JetBrains Mono", monospace;
    font-size: 0.6rem;
    line-height: 1;
    font-weight: 500;
    white-space: nowrap;
    flex-shrink: 0;
    opacity: 0.85;
  }

  .spark-label.min { text-align: right; min-width: 28px; }
  .spark-label.max { text-align: left; min-width: 28px; }
</style>
