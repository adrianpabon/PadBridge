<script lang="ts">
  import { api } from "../api";
  import type { Overview } from "../types";
  import Toggle from "./Toggle.svelte";

  let { overview = $bindable() }: { overview: Overview } = $props();

  function save() {
    api.setPrefs($state.snapshot(overview.prefs));
  }
</script>

<div class="wrap">
  <section class="card">
    <h3>Aplicación</h3>
    <Toggle
      label="Al cerrar, seguir en la bandeja del sistema"
      bind:checked={overview.prefs.closeToTray}
      hint="Los controles solo funcionan mientras PadBridge esté abierto. Sal desde el ícono junto al reloj."
    />
    <Toggle label="Iniciar minimizado en la bandeja" bind:checked={overview.prefs.startMinimized} />
    <div><button class="primary" onclick={save}>Guardar</button></div>
  </section>

  <section class="card">
    <h3>Estado</h3>
    <p>
      Driver ViGEmBus:
      {#if overview.vigemInstalled}<b class="ok">instalado</b>{:else}<b class="bad">no encontrado</b>{/if}
    </p>
    {#if !overview.vigemInstalled}
      <p class="muted small">
        Sin este driver no se pueden crear controles de Xbox virtuales. Instálalo desde
        <b>github.com/nefarius/ViGEmBus/releases</b>.
      </p>
    {/if}
    <p class="muted small">Configuración guardada en: {overview.settingsPath}</p>
  </section>

  <section class="card">
    <h3>Acerca de</h3>
    <p>PadBridge 0.2 — controles PlayStation como controles de Xbox.</p>
    <p class="muted small">
      Usa el driver ViGEmBus de Nefarius para los controles virtuales. Los formatos del DualSense siguen el driver
      hid-playstation de Linux y las notas públicas de efectos de gatillo de Nielk1.
    </p>
  </section>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 720px;
  }
  p {
    margin: 0;
  }
  .ok {
    color: var(--ok);
  }
  .bad {
    color: var(--bad);
  }
</style>
