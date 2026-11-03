<script lang="ts">
  import type { InputEvent, Profile } from "../types";
  import Slider from "./Slider.svelte";
  import StickEditor from "./StickEditor.svelte";

  let { profile = $bindable(), ev }: { profile: Profile; ev: InputEvent | null } = $props();

  const ax = (v: number | undefined) => (v === undefined ? 0 : (v - 127.5) / 127.5);
  const left = $derived<[number, number]>([ax(ev?.raw.lx), -ax(ev?.raw.ly)]);
  const right = $derived<[number, number]>([ax(ev?.raw.rx), -ax(ev?.raw.ry)]);

  const triggers = $derived([
    { label: "L2", key: "l2" as const, raw: (ev?.raw.l2 ?? 0) / 255, out: ev?.outL2 ?? 0 },
    { label: "R2", key: "r2" as const, raw: (ev?.raw.r2 ?? 0) / 255, out: ev?.outR2 ?? 0 },
  ]);
  const pct = (v: number) => Math.round(v * 100) + "%";
</script>

<div class="wrap">
  <p class="muted">
    Ajustes de este control solamente. Para copiarlos a los demás usa <b>Aplicar a todos</b> arriba.
  </p>
  <div class="grid-2">
    <StickEditor label="Stick izquierdo (moverse)" bind:stick={profile.leftStick} raw={left} out={ev?.outLeft} />
    <StickEditor label="Stick derecho (apuntar)" bind:stick={profile.rightStick} raw={right} out={ev?.outRight} />
  </div>

  <div class="grid-2">
    {#each triggers as t (t.key)}
      <section class="card">
        <h2>Gatillo {t.label}</h2>
        <div class="tbar">
          <div class="dz" style="width: {profile[t.key].deadzone * 100}%"></div>
          <div class="sat" style="left: {profile[t.key].max * 100}%"></div>
          <div class="raw" style="width: {t.raw * 100}%"></div>
        </div>
        <div class="row small muted">
          <span>Presionado: {pct(t.raw)}</span><span style="margin-left: auto">El juego recibe: {pct(t.out)}</span>
        </div>
        <Slider
          label="Zona muerta"
          bind:value={profile[t.key].deadzone}
          min={0}
          max={0.6}
          step={0.01}
          format={pct}
          hint="Lo que no llegue a este punto cuenta como suelto."
        />
        <Slider
          label="Punto máximo"
          bind:value={profile[t.key].max}
          min={Math.max(0.3, Math.round((profile[t.key].deadzone + 0.05) * 100) / 100)}
          max={1}
          step={0.01}
          format={pct}
          hint="Bájalo para disparar a fondo sin apretar todo el gatillo (disparo rápido)."
        />
      </section>
    {/each}
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .wrap > p {
    margin: 0;
  }
  .tbar {
    position: relative;
    height: 22px;
    background: var(--bg-2);
    border-radius: 8px;
    overflow: hidden;
  }
  .tbar .raw {
    position: absolute;
    inset: 0 auto 0 0;
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
    opacity: 0.85;
  }
  .tbar .dz {
    position: absolute;
    inset: 0 auto 0 0;
    background: rgba(248, 113, 113, 0.25);
    z-index: 1;
  }
  .tbar .sat {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: #fff;
    z-index: 2;
  }
</style>
