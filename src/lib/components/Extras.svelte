<script lang="ts">
  import { api } from "../api";
  import type { ControllerInfo, Profile } from "../types";
  import Slider from "./Slider.svelte";
  import Toggle from "./Toggle.svelte";

  let { profile = $bindable(), info }: { profile: Profile; info: ControllerInfo } = $props();
</script>

<div class="grid-2">
  <section class="card">
    <h3>Control de Xbox virtual</h3>
    <Toggle
      label="Usar como control de Xbox"
      bind:checked={profile.emulate}
      hint="El juego lo ve como un control de Xbox 360. Apágalo si el juego ya soporta DualSense."
    />
    {#if info.emulationError}
      <p class="err small">{info.emulationError}</p>
    {:else if info.emulating}
      <p class="muted small">Es el control Xbox {info.slot !== null ? info.slot + 1 : "…"}.</p>
    {/if}

    <h3 style="margin-top: 8px">Vibración</h3>
    <Toggle
      label="Vibración del juego"
      bind:checked={profile.rumble.enabled}
      hint="Pasa al DualSense la vibración que el juego le manda al control de Xbox."
    />
    <Slider
      label="Intensidad"
      bind:value={profile.rumble.intensity}
      min={0}
      max={2}
      step={0.05}
      format={(v) => Math.round(v * 100) + "%"}
      disabled={!profile.rumble.enabled}
    />
    <div class="row">
      <button onclick={() => api.testRumble(info.id, 255, 0)}>Probar motor fuerte</button>
      <button onclick={() => api.testRumble(info.id, 0, 255)}>Probar motor suave</button>
    </div>
  </section>

  <section class="card">
    <h3>Panel táctil</h3>
    <Toggle
      label="Usar como mouse"
      bind:checked={profile.touchpad.mouse}
      hint="Desliza el dedo para mover el puntero; el clic del panel es clic izquierdo. Útil en menús de Windows."
    />
    <Slider
      label="Velocidad del puntero"
      bind:value={profile.touchpad.speed}
      min={0.2}
      max={4}
      step={0.1}
      format={(v) => v.toFixed(1) + "×"}
      disabled={!profile.touchpad.mouse}
    />
    <Toggle
      label="Clic del panel = botón Back"
      bind:checked={profile.touchpad.clickAsBack}
      disabled={profile.touchpad.mouse}
      hint="En Halo abre el marcador. El botón Create también es Back."
    />
  </section>
</div>

<style>
  .err {
    color: var(--bad);
    margin: 0;
  }
  p {
    margin: 0;
  }
</style>
