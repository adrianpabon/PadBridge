<script lang="ts">
  import { MODE_HELP, MODE_LABELS, defaultEffect } from "../presets";
  import type { TriggerEffect, TriggerMode } from "../types";
  import Slider from "./Slider.svelte";

  interface Props {
    label: string;
    effect: TriggerEffect;
    /** Live trigger pressure, 0-255. */
    pressure: number;
  }
  let { label, effect = $bindable(), pressure }: Props = $props();

  const MODES = Object.keys(MODE_LABELS) as TriggerMode[];

  function setMode(mode: TriggerMode) {
    if (mode !== effect.mode) effect = defaultEffect(mode);
  }

  // Per-zone strength (0-8) for the strip, an approximation of how each mode feels.
  const zones = $derived.by((): number[] => {
    const z = new Array(10).fill(0);
    const e = effect;
    const span = (a: number, b: number, v: (i: number) => number) => {
      for (let i = a; i <= Math.min(9, b); i++) z[i] = v(i);
    };
    switch (e.mode) {
      case "resistance":
        span(e.start, 9, () => e.strength);
        break;
      case "slope":
        span(e.start, 9, (i) =>
          i >= e.end ? e.endStrength : e.startStrength + ((e.endStrength - e.startStrength) * (i - e.start)) / (e.end - e.start),
        );
        break;
      case "weapon":
        span(e.start, e.end - 1, () => e.strength);
        break;
      case "vibration":
        span(e.start, 9, (i) => (i % 2 ? e.amplitude : e.amplitude * 0.6));
        break;
      case "bow":
        span(e.start, e.end, (i) => (e.strength * (i - e.start + 1)) / (e.end - e.start + 1));
        break;
      case "galloping":
        span(e.start, e.end, (i) => (i % 3 === 0 ? 6 : 2));
        break;
      case "machine":
        span(e.start, e.end, (i) => (i % 2 ? e.ampA + 1 : e.ampB + 1));
        break;
      case "zones":
        e.strengths.forEach((s, i) => (z[i] = s));
        break;
    }
    return z;
  });

  const pos = (v: number) => `${Math.round(v * 10)}%`;
  const pct = (v: number) => `${Math.round((v / 9) * 100)}%`;
</script>

<section class="card trig">
  <div class="head">
    <h2>{label}</h2>
    <select value={effect.mode} onchange={(e) => setMode(e.currentTarget.value as TriggerMode)}>
      {#each MODES as m}<option value={m}>{MODE_LABELS[m]}</option>{/each}
    </select>
  </div>
  <p class="muted small">{MODE_HELP[effect.mode]}</p>

  <div class="strip" title="Fuerza por zona; la línea es la posición actual del gatillo">
    {#each zones as s, i}
      <div class="zone"><div class="bar" style="height: {(s / 8) * 100}%"></div><span>{i}</span></div>
    {/each}
    <div class="cursor" style="left: {(pressure / 255) * 100}%"></div>
  </div>
  <div class="row small muted"><span>Suelto</span><span style="margin-left: auto">Al fondo</span></div>

  <div class="params">
    {#if effect.mode === "resistance"}
      <Slider label="Empieza en" bind:value={effect.start} min={0} max={9} format={pct} />
      <Slider label="Fuerza" bind:value={effect.strength} min={1} max={8} />
    {:else if effect.mode === "slope"}
      <Slider label="Empieza en" bind:value={effect.start} min={0} max={8} format={pct} />
      <Slider label="Termina en" bind:value={effect.end} min={effect.start + 1} max={9} format={pct} />
      <Slider label="Fuerza al inicio" bind:value={effect.startStrength} min={1} max={8} />
      <Slider label="Fuerza al final" bind:value={effect.endStrength} min={1} max={8} />
    {:else if effect.mode === "weapon"}
      <Slider label="Empieza en" bind:value={effect.start} min={2} max={7} format={pct} />
      <Slider label="Cede en" bind:value={effect.end} min={effect.start + 1} max={8} format={pct} />
      <Slider label="Fuerza" bind:value={effect.strength} min={1} max={8} />
    {:else if effect.mode === "vibration"}
      <Slider label="Empieza en" bind:value={effect.start} min={0} max={9} format={pct} />
      <Slider label="Intensidad" bind:value={effect.amplitude} min={1} max={8} />
      <Slider label="Frecuencia" bind:value={effect.frequency} min={1} max={255} format={(v) => v + " Hz"} />
    {:else if effect.mode === "bow"}
      <Slider label="Empieza en" bind:value={effect.start} min={0} max={7} format={pct} />
      <Slider label="Se suelta en" bind:value={effect.end} min={effect.start + 1} max={8} format={pct} />
      <Slider label="Tensión" bind:value={effect.strength} min={1} max={8} />
      <Slider label="Fuerza al soltar" bind:value={effect.snap} min={1} max={8} />
    {:else if effect.mode === "galloping"}
      <Slider label="Empieza en" bind:value={effect.start} min={0} max={8} format={pct} />
      <Slider label="Termina en" bind:value={effect.end} min={effect.start + 1} max={9} format={pct} />
      <Slider label="Primer golpe" bind:value={effect.firstFoot} min={0} max={6} />
      <Slider label="Segundo golpe" bind:value={effect.secondFoot} min={effect.firstFoot + 1} max={7} />
      <Slider label="Frecuencia" bind:value={effect.frequency} min={1} max={40} format={(v) => v + " Hz"} />
    {:else if effect.mode === "machine"}
      <Slider label="Empieza en" bind:value={effect.start} min={1} max={8} format={pct} />
      <Slider label="Termina en" bind:value={effect.end} min={effect.start + 1} max={9} format={pct} />
      <Slider label="Fuerza A" bind:value={effect.ampA} min={0} max={7} />
      <Slider label="Fuerza B" bind:value={effect.ampB} min={0} max={7} />
      <Slider label="Frecuencia" bind:value={effect.frequency} min={1} max={60} format={(v) => v + " Hz"} />
      <Slider label="Periodo" bind:value={effect.period} min={0} max={20} />
    {:else if effect.mode === "zones"}
      <div class="zone-sliders">
        {#each effect.strengths as _, i}
          <label>
            <input type="range" min="0" max="8" bind:value={effect.strengths[i]} />
            <span>{effect.strengths[i]}</span>
          </label>
        {/each}
      </div>
    {/if}
  </div>
  <p class="muted small">Pruébalo apretando {label}: el cambio se aplica al instante. ({pos(pressure / 255)})</p>
</section>

<style>
  .trig {
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .strip {
    position: relative;
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    gap: 4px;
    height: 90px;
    padding: 6px;
    background: var(--bg-2);
    border-radius: 10px;
  }
  .zone {
    position: relative;
    display: flex;
    align-items: flex-end;
    background: #1c2232;
    border-radius: 5px;
    overflow: hidden;
  }
  .zone span {
    position: absolute;
    top: 3px;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 10px;
    color: #4a5570;
  }
  .bar {
    width: 100%;
    background: linear-gradient(180deg, var(--accent-2), var(--accent));
    border-radius: 4px 4px 0 0;
    transition: height 0.15s;
  }
  .cursor {
    position: absolute;
    top: 2px;
    bottom: 2px;
    width: 3px;
    margin-left: -1px;
    background: #fff;
    box-shadow: 0 0 10px #fff;
    border-radius: 3px;
  }
  .params {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 18px;
  }
  .zone-sliders {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    gap: 4px;
  }
  .zone-sliders label {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  .zone-sliders input {
    writing-mode: vertical-lr;
    direction: rtl;
    height: 90px;
    width: 18px;
    accent-color: var(--accent);
  }
</style>
