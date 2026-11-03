<script lang="ts" generics="T extends string | number">
  interface Props {
    options: { value: T; label: string }[];
    value: T;
  }
  let { options, value = $bindable() }: Props = $props();
</script>

<div class="seg">
  {#each options as o (o.value)}
    <button class:on={o.value === value} onclick={() => (value = o.value)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: 9px;
    padding: 3px;
    gap: 2px;
    flex-wrap: wrap;
  }
  button {
    border: none;
    background: transparent;
    padding: 6px 12px;
    border-radius: 6px;
    color: var(--muted);
  }
  button:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  button.on {
    background: var(--accent);
    color: #fff;
  }
</style>
