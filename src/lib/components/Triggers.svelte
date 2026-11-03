<script lang="ts">
  import { PRESETS } from "../presets";
  import type { Profile } from "../types";
  import TriggerEditor from "./TriggerEditor.svelte";

  let { profile = $bindable(), l2, r2 }: { profile: Profile; l2: number; r2: number } = $props();

  const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
  const active = $derived(
    PRESETS.find((p) => same(p.left, profile.leftTrigger) && same(p.right, profile.rightTrigger))?.name,
  );
</script>

<div class="wrap">
  <section class="card">
    <h3>Preajustes</h3>
    <div class="presets">
      {#each PRESETS as p}
        <button
          class:on={active === p.name}
          onclick={() => {
            profile.leftTrigger = structuredClone(p.left);
            profile.rightTrigger = structuredClone(p.right);
          }}>{p.name}</button
        >
      {/each}
    </div>
    <p class="muted small">
      Los gatillos adaptativos los pone PadBridge, no el juego: el efecto se siente igual en cualquier juego, aunque no
      tenga soporte para DualSense.
    </p>
  </section>

  <div class="grid-2">
    <TriggerEditor label="L2" bind:effect={profile.leftTrigger} pressure={l2} />
    <TriggerEditor label="R2" bind:effect={profile.rightTrigger} pressure={r2} />
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .presets button.on {
    background: var(--accent);
    border-color: var(--accent);
  }
</style>
