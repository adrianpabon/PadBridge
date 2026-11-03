<script lang="ts">
  import { onMount } from "svelte";
  import { api, events } from "./lib/api";
  import type { ControllerInfo, InputEvent, Overview, Profile } from "./lib/types";
  import LiveInput from "./lib/components/LiveInput.svelte";
  import Lights from "./lib/components/Lights.svelte";
  import Triggers from "./lib/components/Triggers.svelte";
  import Sensitivity from "./lib/components/Sensitivity.svelte";
  import Extras from "./lib/components/Extras.svelte";
  import AppSettings from "./lib/components/AppSettings.svelte";

  type Tab = "live" | "triggers" | "sensitivity" | "lights" | "extras";
  const TABS: { id: Tab; label: string }[] = [
    { id: "live", label: "En vivo" },
    { id: "triggers", label: "Gatillos" },
    { id: "sensitivity", label: "Sensibilidad" },
    { id: "lights", label: "Luces" },
    { id: "extras", label: "Vibración y más" },
  ];

  let overview = $state<Overview | null>(null);
  let controllers = $state<ControllerInfo[]>([]);
  let selectedId = $state<string | null>(null);
  let showSettings = $state(false);
  let tab = $state<Tab>("live");
  let profile = $state<Profile | null>(null);
  let ev = $state<InputEvent | null>(null);
  let toast = $state<string | null>(null);

  const selected = $derived(controllers.find((c) => c.id === selectedId) ?? null);
  const isXbox = $derived(selected?.kind === "xbox");
  const XBOX_TABS: { id: Tab; label: string }[] = [
    { id: "live", label: "En vivo" },
    { id: "extras", label: "Vibración" },
  ];
  const tabs = $derived(isXbox ? XBOX_TABS : TABS);
  $effect(() => {
    if (!tabs.some((t) => t.id === tab)) tab = "live";
  });
  const CONNECTION: Record<string, string> = { usb: "USB", bluetooth: "Bluetooth", wireless: "Inalámbrico" };

  onMount(() => {
    const unlisten = [
      events.controllers((list) => {
        controllers = list;
        if (!selectedId || !list.some((c) => c.id === selectedId)) selectedId = list[0]?.id ?? null;
      }),
      events.overview((o) => (overview = o)),
      events.input((e) => {
        if (e.id === selectedId) ev = e;
      }),
    ];
    api.overview().then((o) => {
      overview = o;
      controllers = o.controllers;
      selectedId = o.controllers[0]?.id ?? null;
    });
    const onVisibility = () => api.watchInput(document.hidden ? null : selectedId);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      unlisten.forEach((p) => p.then((f) => f()));
      document.removeEventListener("visibilitychange", onVisibility);
    };
  });

  // Load the selected controller's profile and stream its input.
  let lastSaved = "";
  $effect(() => {
    const id = selectedId;
    ev = null;
    profile = null;
    api.watchInput(id);
    if (!id || id.startsWith("xbox-")) return; // Xbox controllers have no profile
    api.profile(id).then((p) => {
      if (selectedId !== id) return;
      lastSaved = JSON.stringify(p);
      profile = p;
    });
  });

  // Every change goes to the controller (and to disk) right away, debounced while a slider is dragged.
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (!profile || !selectedId) return;
    const json = JSON.stringify(profile);
    if (json === lastSaved) return;
    const id = selectedId;
    const snap = $state.snapshot(profile);
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      lastSaved = json;
      api.setProfile(id, snap);
    }, 60);
  });

  function flash(msg: string) {
    toast = msg;
    setTimeout(() => (toast = null), 2200);
  }

  async function applyToAll() {
    if (!selectedId) return;
    await api.applyToAll(selectedId);
    flash("Configuración copiada a todos los controles");
  }

  async function reset() {
    if (!selectedId || !confirm("¿Volver a la configuración por defecto de este control?")) return;
    const p = await api.resetProfile(selectedId);
    lastSaved = JSON.stringify(p);
    profile = p;
    flash("Configuración restablecida");
  }

  const batteryColor = (b: number) => (b > 50 ? "var(--ok)" : b > 20 ? "var(--warn)" : "var(--bad)");
</script>

<div class="app">
  <aside>
    <div class="brand">
      <div class="logo"></div>
      <div>
        <h1>PadBridge</h1>
        <span class="muted small">PlayStation → Xbox</span>
      </div>
    </div>

    <h3 class="side-title">Controles</h3>
    <div class="list">
      {#each controllers as c (c.id)}
        <button
          class="ctrl"
          class:sel={c.id === selectedId && !showSettings}
          onclick={() => {
            selectedId = c.id;
            showSettings = false;
          }}
        >
          <div class="ctrl-top">
            <span class="name">{c.name}</span>
            {#if c.kind === "xbox" && c.slot !== null}
              <span class="badge native">Xbox {c.slot + 1}</span>
            {:else if c.emulating && c.slot !== null}
              <span class="badge">Xbox {c.slot + 1}</span>
            {:else if c.emulationError}
              <span class="badge bad">Error</span>
            {:else if !c.emulating}
              <span class="badge off">Sin emular</span>
            {/if}
          </div>
          <div class="ctrl-meta">
            <span>{CONNECTION[c.connection] ?? c.connection}</span>
            {#if c.battery !== null}
              <span class="bat">
                <span class="bat-body"><span class="bat-fill" style="width: {c.battery}%; background: {batteryColor(c.battery)}"></span></span>
                {c.battery}%{c.charging === "charging" ? " ⚡" : ""}
              </span>
            {:else}
              <span class="bat" title="Este control no informa su batería">Batería —</span>
            {/if}
          </div>
        </button>
      {:else}
        <p class="muted small empty">
          Conecta un DualSense por USB o Bluetooth. Aparecerá aquí solo.
        </p>
      {/each}
    </div>

    <button class="settings-btn" class:sel={showSettings} onclick={() => (showSettings = true)}>⚙ Ajustes</button>
  </aside>

  <main>
    {#if overview && !overview.vigemInstalled}
      <div class="banner bad">
        No se encontró el driver <b>ViGEmBus</b>: sin él los juegos no ven los controles. Instálalo desde
        github.com/nefarius/ViGEmBus/releases y reinicia PadBridge.
      </div>
    {/if}
    {#if overview && overview.conflicts.length}
      <div class="banner warn">
        <b>{overview.conflicts.join(", ")}</b> está abierto. Ciérralo: si no, cada control aparece dos veces en los juegos.
      </div>
    {/if}

    {#if showSettings && overview}
      <h2 class="page-title">Ajustes</h2>
      <AppSettings bind:overview />
    {:else if selected && isXbox}
      <header>
        <div>
          <h2>{selected.name}</h2>
          <span class="muted small">Control Xbox {selected.slot !== null ? selected.slot + 1 : ""} · lo lee el juego directamente</span>
        </div>
        <div class="row">
          <button onclick={() => api.identify(selected.id)} title="Hace vibrar este control">Identificar</button>
        </div>
      </header>
      <nav>
        {#each tabs as t}
          <button class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
        {/each}
      </nav>
      <div class="content">
        {#if tab === "live"}
          <section class="card"><LiveInput {ev} xbox /></section>
        {:else}
          <section class="card" style="max-width: 560px">
            <h3>Vibración</h3>
            <div class="row">
              <button onclick={() => api.testRumble(selected.id, 255, 0)}>Probar motor fuerte</button>
              <button onclick={() => api.testRumble(selected.id, 0, 255)}>Probar motor suave</button>
            </div>
            <p class="muted small" style="margin: 0">
              Los controles de Xbox no tienen barra de luz ni gatillos adaptativos, y el juego los lee sin pasar por
              PadBridge, así que su sensibilidad se ajusta en el juego.
            </p>
          </section>
        {/if}
      </div>
    {:else if selected && profile}
      <header>
        <div>
          <h2>{selected.name}</h2>
          <span class="muted small">{selected.id}</span>
        </div>
        <div class="row">
          <button onclick={() => api.identify(selected.id)} title="Hace parpadear y vibrar este control">Identificar</button>
          <button onclick={applyToAll} disabled={controllers.length < 2}>Aplicar a todos</button>
          <button onclick={reset}>Restablecer</button>
        </div>
      </header>

      <nav>
        {#each tabs as t}
          <button class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
        {/each}
      </nav>

      <div class="content">
        {#if tab === "live"}
          <section class="card"><LiveInput {ev} /></section>
        {:else if tab === "triggers"}
          <Triggers bind:profile l2={ev?.raw.l2 ?? 0} r2={ev?.raw.r2 ?? 0} />
        {:else if tab === "sensitivity"}
          <Sensitivity bind:profile {ev} />
        {:else if tab === "lights"}
          <Lights bind:profile />
        {:else if tab === "extras"}
          <Extras bind:profile info={selected} />
        {/if}
      </div>
    {:else if !selected}
      <div class="placeholder">
        <div class="logo big"></div>
        <h2>No hay controles conectados</h2>
        <p class="muted">
          Conecta tus DualSense por USB o Bluetooth.<br />Cada uno aparecerá como un control de Xbox para los juegos.
        </p>
      </div>
    {/if}
  </main>

  {#if toast}<div class="toast">{toast}</div>{/if}
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 250px 1fr;
    height: 100%;
  }
  aside {
    background: var(--bg-2);
    border-right: 1px solid var(--line);
    padding: 18px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
  }
  .brand {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 0 4px 6px;
  }
  .brand h1 {
    font-size: 17px;
  }
  .logo {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    box-shadow: 0 4px 18px rgba(79, 125, 255, 0.35);
  }
  .logo.big {
    width: 72px;
    height: 72px;
    border-radius: 20px;
  }
  .side-title {
    padding: 0 4px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    flex: 1;
  }
  .empty {
    padding: 0 4px;
    margin: 0;
  }
  .ctrl {
    text-align: left;
    background: var(--panel);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .ctrl.sel {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent) inset;
  }
  .ctrl-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
  }
  .name {
    font-weight: 600;
  }
  .badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 20px;
    background: rgba(52, 211, 153, 0.15);
    color: var(--ok);
    white-space: nowrap;
  }
  .badge.bad {
    background: rgba(248, 113, 113, 0.15);
    color: var(--bad);
  }
  .badge.native {
    background: rgba(79, 125, 255, 0.15);
    color: #93b0ff;
  }
  .badge.off {
    background: #252d40;
    color: var(--muted);
  }
  .ctrl-meta {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--muted);
  }
  .bat {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .bat-body {
    width: 26px;
    height: 10px;
    border: 1px solid #3a4560;
    border-radius: 3px;
    padding: 1px;
    display: flex;
  }
  .bat-fill {
    border-radius: 1px;
  }
  .settings-btn {
    text-align: left;
  }
  .settings-btn.sel {
    border-color: var(--accent);
  }
  main {
    padding: 22px 28px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .page-title {
    margin-bottom: 4px;
  }
  nav {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--line);
  }
  nav button {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 10px 14px;
    color: var(--muted);
  }
  nav button:hover {
    background: none;
    color: var(--text);
  }
  nav button.on {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .banner {
    padding: 12px 16px;
    border-radius: 10px;
    border: 1px solid;
  }
  .banner.bad {
    background: rgba(248, 113, 113, 0.08);
    border-color: rgba(248, 113, 113, 0.4);
  }
  .banner.warn {
    background: rgba(251, 191, 36, 0.08);
    border-color: rgba(251, 191, 36, 0.4);
  }
  .placeholder {
    margin: auto;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .toast {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 10px 18px;
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }
</style>
