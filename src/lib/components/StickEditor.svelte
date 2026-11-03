<script lang="ts">
  import type { StickSettings } from "../types";
  import Slider from "./Slider.svelte";
  import Toggle from "./Toggle.svelte";

  interface Props {
    label: string;
    stick: StickSettings;
    /** Live raw position (-1..1, y up) and what the game gets. */
    raw: [number, number];
    out: [number, number] | undefined;
  }
  let { label, stick = $bindable(), raw, out }: Props = $props();

  // Same response as src-tauri/src/mapping.rs shape_stick, along one axis.
  function response(r: number, s: StickSettings): number {
    const dz = Math.min(0.5, Math.max(0, s.deadzone));
    if (r <= dz) return 0;
    let n = Math.min(1, (r - dz) / (1 - dz));
    n = Math.pow(n, Math.min(4, Math.max(0.2, s.curve)));
    n = Math.min(1, n * s.sensitivity);
    const anti = Math.min(0.5, Math.max(0, s.antiDeadzone));
    return anti + (1 - anti) * n;
  }

  const W = 160;
  const path = $derived.by(() => {
    let d = "";
    for (let i = 0; i <= 80; i++) {
      const x = i / 80;
      d += `${i ? "L" : "M"}${(x * W).toFixed(1)} ${(W - response(x, stick) * W).toFixed(1)}`;
    }
    return d;
  });
  const rawR = $derived(Math.min(1, Math.hypot(raw[0], raw[1])));

  const PRESETS: { name: string; s: Partial<StickSettings> }[] = [
    { name: "Precisión", s: { sensitivity: 0.8, curve: 1.8, deadzone: 0.06, antiDeadzone: 0 } },
    { name: "Normal", s: { sensitivity: 1, curve: 1, deadzone: 0.06, antiDeadzone: 0 } },
    { name: "Rápida", s: { sensitivity: 1.4, curve: 0.8, deadzone: 0.06, antiDeadzone: 0.05 } },
    { name: "Máxima", s: { sensitivity: 2, curve: 0.6, deadzone: 0.05, antiDeadzone: 0.1 } },
  ];
</script>

<section class="card">
  <div class="head">
    <h2>{label}</h2>
    <div class="row">
      {#each PRESETS as p}
        <button class="small-btn" onclick={() => Object.assign(stick, p.s)}>{p.name}</button>
      {/each}
    </div>
  </div>

  <div class="viz">
    <svg viewBox="-6 -6 {W + 12} {W + 12}" class="curve">
      <rect x="0" y="0" width={W} height={W} class="frame" />
      <line x1="0" y1={W} x2={W} y2="0" class="diag" />
      <path d={path} class="resp" />
      <circle cx={rawR * W} cy={W - response(rawR, stick) * W} r="5" class="now" />
      <text x="4" y={W - 4} class="ax">entrada →</text>
    </svg>
    <svg viewBox="-60 -60 120 120" class="stick">
      <circle r="50" class="base" />
      <circle r={stick.deadzone * 50} class="dz" />
      <circle cx={raw[0] * 50} cy={-raw[1] * 50} r="6" class="raw" />
      {#if out}<circle cx={out[0] * 50} cy={-out[1] * 50} r="4.5" class="out" />{/if}
    </svg>
  </div>

  <Slider
    label="Sensibilidad"
    bind:value={stick.sensitivity}
    min={0.25}
    max={3}
    step={0.05}
    format={(v) => v.toFixed(2) + "×"}
    hint="Más alta: llegas al máximo con menos recorrido."
  />
  <Slider
    label="Curva de respuesta"
    bind:value={stick.curve}
    min={0.3}
    max={3}
    step={0.05}
    format={(v) => (Math.abs(v - 1) < 0.01 ? "lineal" : v > 1 ? `suave (${v.toFixed(2)})` : `rápida (${v.toFixed(2)})`)}
    hint="Suave: más precisión cerca del centro (apuntar). Rápida: responde antes."
  />
  <Slider
    label="Zona muerta"
    bind:value={stick.deadzone}
    min={0}
    max={0.4}
    step={0.01}
    format={(v) => Math.round(v * 100) + "%"}
    hint="Súbela si el personaje se mueve solo (drift)."
  />
  <Slider
    label="Anti-zona muerta"
    bind:value={stick.antiDeadzone}
    min={0}
    max={0.4}
    step={0.01}
    format={(v) => Math.round(v * 100) + "%"}
    hint="Compensa la zona muerta propia del juego para que responda al primer toque."
  />
  <div class="toggles">
    <Toggle label="Invertir eje X" bind:checked={stick.invertX} />
    <Toggle label="Invertir eje Y" bind:checked={stick.invertY} />
  </div>
</section>

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .small-btn {
    padding: 4px 10px;
    font-size: 12px;
  }
  .viz {
    display: flex;
    gap: 16px;
    justify-content: center;
  }
  .curve,
  .stick {
    width: 150px;
    height: 150px;
    background: var(--bg-2);
    border-radius: 10px;
  }
  .frame {
    fill: none;
    stroke: #273044;
  }
  .diag {
    stroke: #2a3348;
    stroke-dasharray: 4 4;
  }
  .resp {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3;
  }
  .now {
    fill: #fff;
  }
  .ax {
    font-size: 9px;
    fill: #4a5570;
  }
  .base {
    fill: #141926;
    stroke: #3a4560;
  }
  .dz {
    fill: rgba(248, 113, 113, 0.15);
    stroke: rgba(248, 113, 113, 0.5);
  }
  .raw {
    fill: #8b95aa;
  }
  .out {
    fill: var(--accent);
    stroke: #fff;
  }
  .toggles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
</style>
