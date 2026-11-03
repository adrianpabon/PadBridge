<script lang="ts">
  interface Props {
    label: string;
    checked: boolean;
    hint?: string;
    disabled?: boolean;
  }
  let { label, checked = $bindable(), hint, disabled = false }: Props = $props();
</script>

<label class="toggle" class:disabled>
  <span class="text">
    <span>{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" bind:checked {disabled} />
  <span class="track"><span class="knob"></span></span>
</label>

<style>
  .toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    cursor: pointer;
  }
  .toggle.disabled {
    opacity: 0.45;
    cursor: default;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .hint {
    font-size: 12px;
    color: var(--muted);
  }
  input {
    display: none;
  }
  .track {
    flex: none;
    width: 40px;
    height: 22px;
    border-radius: 22px;
    background: #2a3348;
    position: relative;
    transition: background 0.15s;
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.15s;
  }
  input:checked + .track {
    background: var(--accent);
  }
  input:checked + .track .knob {
    transform: translateX(18px);
  }
</style>
