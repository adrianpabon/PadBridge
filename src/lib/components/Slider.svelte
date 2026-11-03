<script lang="ts">
  interface Props {
    label: string;
    value: number;
    min: number;
    max: number;
    step?: number;
    format?: (v: number) => string;
    hint?: string;
    disabled?: boolean;
  }
  let { label, value = $bindable(), min, max, step = 1, format = (v) => String(v), hint, disabled = false }: Props =
    $props();

  const pct = $derived(((value - min) / (max - min)) * 100);
</script>

<label class="slider" class:disabled title={hint}>
  <span class="top">
    <span>{label}</span>
    <span class="val">{format(value)}</span>
  </span>
  <input type="range" {min} {max} {step} {disabled} bind:value style="--pct: {pct}%" />
  {#if hint}<span class="hint">{hint}</span>{/if}
</label>

<style>
  .slider {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .slider.disabled {
    opacity: 0.45;
  }
  .top {
    display: flex;
    justify-content: space-between;
  }
  .val {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .hint {
    font-size: 12px;
    color: var(--muted);
  }
  input {
    appearance: none;
    width: 100%;
    height: 6px;
    border-radius: 6px;
    background: linear-gradient(90deg, var(--accent) 0 var(--pct), #2a3348 var(--pct) 100%);
    outline: none;
    margin: 4px 0;
  }
  input::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    border: 3px solid var(--accent);
    cursor: pointer;
  }
</style>
