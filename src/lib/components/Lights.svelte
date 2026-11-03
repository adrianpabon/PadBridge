<script lang="ts">
  import type { Profile } from "../types";
  import Segmented from "./Segmented.svelte";
  import Slider from "./Slider.svelte";

  let { profile = $bindable() }: { profile: Profile } = $props();

  const SWATCHES: [number, number, number][] = [
    [0, 90, 255],
    [124, 58, 237],
    [236, 72, 153],
    [239, 68, 68],
    [249, 115, 22],
    [250, 204, 21],
    [34, 197, 94],
    [6, 182, 212],
    [255, 255, 255],
  ];

  const hex = (c: number[]) => "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("");
  function setHex(h: string) {
    profile.lightbar.color = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
  }

  // What the bar shows now, for the preview.
  const preview = $derived.by(() => {
    const lb = profile.lightbar;
    if (lb.mode === "off") return "#000";
    if (lb.mode === "rainbow") return "conic-gradient(red, yellow, lime, cyan, blue, magenta, red)";
    if (lb.mode === "battery") return "linear-gradient(90deg, #ef4444, #facc15, #22c55e)";
    return hex(lb.color);
  });

  function toggleLed(i: number) {
    profile.playerLeds.mode = "custom";
    profile.playerLeds.custom ^= 1 << i;
  }
</script>

<div class="grid-2">
  <section class="card">
    <h3>Barra de luz</h3>
    <div class="preview" style="background: {preview}; opacity: {profile.lightbar.mode === 'off' ? 1 : 0.35 + profile.lightbar.brightness * 0.65}"></div>
    <Segmented
      bind:value={profile.lightbar.mode}
      options={[
        { value: "static", label: "Color" },
        { value: "rainbow", label: "Arcoíris" },
        { value: "battery", label: "Batería" },
        { value: "off", label: "Apagada" },
      ]}
    />
    {#if profile.lightbar.mode === "static"}
      <div class="swatches">
        {#each SWATCHES as c}
          <button
            class="sw"
            class:sel={hex(c) === hex(profile.lightbar.color)}
            style="background: {hex(c)}"
            aria-label={hex(c)}
            onclick={() => (profile.lightbar.color = [...c] as [number, number, number])}
          ></button>
        {/each}
        <label class="custom" title="Color personalizado">
          <input type="color" value={hex(profile.lightbar.color)} oninput={(e) => setHex(e.currentTarget.value)} />
          <span>+</span>
        </label>
      </div>
    {/if}
    {#if profile.lightbar.mode === "rainbow"}
      <Slider label="Velocidad" bind:value={profile.lightbar.speed} min={0.05} max={2} step={0.05} format={(v) => v.toFixed(2) + " vueltas/s"} />
    {/if}
    {#if profile.lightbar.mode === "battery"}
      <p class="muted small">Rojo con poca batería, verde con la batería llena.</p>
    {/if}
    {#if profile.lightbar.mode !== "off"}
      <Slider label="Brillo" bind:value={profile.lightbar.brightness} min={0} max={1} step={0.01} format={(v) => Math.round(v * 100) + "%"} />
    {/if}
  </section>

  <section class="card">
    <h3>LEDs de jugador</h3>
    <div class="leds">
      {#each [0, 1, 2, 3, 4] as i}
        <button
          class="led"
          class:lit={profile.playerLeds.mode === "custom" && (profile.playerLeds.custom & (1 << i)) !== 0}
          aria-label={"LED " + (i + 1)}
          onclick={() => toggleLed(i)}
        ></button>
      {/each}
    </div>
    <Segmented
      bind:value={profile.playerLeds.mode}
      options={[
        { value: "auto", label: "Número de control" },
        { value: "custom", label: "Personalizado" },
        { value: "off", label: "Apagados" },
      ]}
    />
    <p class="muted small">
      {#if profile.playerLeds.mode === "auto"}
        Muestran qué número de control Xbox le tocó (1 a 4), para saber quién es quién en el juego.
      {:else if profile.playerLeds.mode === "custom"}
        Toca los LEDs de arriba para encenderlos o apagarlos.
      {:else}
        Los LEDs quedan apagados.
      {/if}
    </p>
    <span>Brillo de los LEDs</span>
    <Segmented
      bind:value={profile.playerLeds.brightness}
      options={[
        { value: 0, label: "Alto" },
        { value: 1, label: "Medio" },
        { value: 2, label: "Bajo" },
      ]}
    />

    <h3 style="margin-top: 8px">LED del micrófono</h3>
    <Segmented
      bind:value={profile.micLed}
      options={[
        { value: "off", label: "Apagado" },
        { value: "on", label: "Encendido" },
        { value: "pulse", label: "Parpadeo" },
      ]}
    />
  </section>
</div>

<style>
  .preview {
    height: 18px;
    border-radius: 18px;
    box-shadow: 0 0 24px rgba(79, 125, 255, 0.25);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .sw {
    width: 30px;
    height: 30px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid transparent;
  }
  .sw.sel {
    border-color: #fff;
    box-shadow: 0 0 0 2px var(--accent);
  }
  .custom {
    position: relative;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: 2px dashed var(--line);
    display: grid;
    place-items: center;
    color: var(--muted);
    cursor: pointer;
    overflow: hidden;
  }
  .custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .leds {
    display: flex;
    gap: 14px;
    justify-content: center;
    padding: 6px 0;
  }
  .led {
    width: 26px;
    height: 10px;
    padding: 0;
    border-radius: 6px;
    background: #2a3348;
    border: none;
  }
  .led.lit {
    background: #fff;
    box-shadow: 0 0 12px #fff;
  }
</style>
