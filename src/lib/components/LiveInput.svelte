<script lang="ts">
  import { BTN, type InputEvent } from "../types";

  let { ev, xbox = false }: { ev: InputEvent | null; xbox?: boolean } = $props();

  // Bottom, right, left, top face buttons
  const FACES = $derived(
    xbox
      ? { top: ["Y", "#facc15"], right: ["B", "#f87171"], bottom: ["A", "#4ade80"], left: ["X", "#60a5fa"] }
      : { top: ["△", "#34d399"], right: ["○", "#f87171"], bottom: ["✕", "#60a5fa"], left: ["□", "#f472b6"] },
  );

  const raw = $derived(ev?.raw);
  const on = (bit: number) => !!raw && (raw.buttons & bit) !== 0;
  // Raw stick (0-255) to -1..1, y up like the output.
  const ax = (v: number | undefined) => (v === undefined ? 0 : (v - 127.5) / 127.5);

  const STICK_R = 34;

  /** A bar centred at 0 for a signed value, scaled to -1..1. */
  function signed(v: number, scale: number): string {
    const n = Math.max(-1, Math.min(1, v / scale));
    return `left: ${50 + Math.min(0, n) * 50}%; width: ${Math.abs(n) * 50}%`;
  }
</script>

<div class="live">
  <svg viewBox="0 0 640 400" class="pad">
    <defs>
      <linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#2a3246" />
        <stop offset="1" stop-color="#1b2131" />
      </linearGradient>
    </defs>

    <!-- triggers: fill = how far the trigger is pressed -->
    {#each [{ x: 110, v: raw?.l2 ?? 0, label: xbox ? "LT" : "L2" }, { x: 450, v: raw?.r2 ?? 0, label: xbox ? "RT" : "R2" }] as t}
      <rect x={t.x} y="8" width="80" height="34" rx="8" class="slot" />
      <rect x={t.x} y="8" width={(80 * t.v) / 255} height="34" rx="8" class="fill" />
      <text x={t.x + 40} y="30" class="lbl">{t.label} {Math.round((t.v / 255) * 100)}%</text>
    {/each}
    <rect x="104" y="50" width="92" height="22" rx="8" class="btn" class:on={on(BTN.L1)} />
    <text x="150" y="65" class="lbl">{xbox ? "LB" : "L1"}</text>
    <rect x="444" y="50" width="92" height="22" rx="8" class="btn" class:on={on(BTN.R1)} />
    <text x="490" y="65" class="lbl">{xbox ? "RB" : "R1"}</text>

    <!-- body -->
    <path
      d="M120 80 Q 70 80 52 140 L 18 300 Q 8 370 70 385 Q 120 395 160 330 L 190 285 L 450 285 L 480 330 Q 520 395 570 385 Q 632 370 622 300 L 588 140 Q 570 80 520 80 Z"
      fill="url(#body)"
      stroke="#323c55"
      stroke-width="2"
    />

    <!-- touchpad with live touch points -->
    <rect x="218" y="88" width="204" height="110" rx="14" class="btn" class:on={on(BTN.TOUCHPAD)} />
    {#if xbox}
      <text x="320" y="150" class="lbl" style="font-size: 26px; fill: #3a4560">XBOX</text>
    {/if}
    {#if raw && !xbox}
      {#each raw.touch as t}
        {#if t.active}
          <circle cx={218 + (t.x / 1920) * 204} cy={88 + (t.y / 1080) * 110} r="9" class="touch" />
        {/if}
      {/each}
    {/if}

    <!-- create / options / PS / mute -->
    <rect x="186" y="96" width="16" height="30" rx="6" class="btn" class:on={on(BTN.CREATE)} />
    <rect x="438" y="96" width="16" height="30" rx="6" class="btn" class:on={on(BTN.OPTIONS)} />
    <circle cx="320" cy="240" r="14" class="btn" class:on={on(BTN.PS)} />
    <rect x="306" y="262" width="28" height="10" rx="5" class="btn" class:on={on(BTN.MUTE)} />

    <!-- d-pad -->
    <rect x="106" y="112" width="28" height="34" rx="5" class="btn" class:on={on(BTN.UP)} />
    <rect x="106" y="180" width="28" height="34" rx="5" class="btn" class:on={on(BTN.DOWN)} />
    <rect x="68" y="149" width="34" height="28" rx="5" class="btn" class:on={on(BTN.LEFT)} />
    <rect x="138" y="149" width="34" height="28" rx="5" class="btn" class:on={on(BTN.RIGHT)} />

    <!-- face buttons -->
    {#each [{ x: 520, y: 122, b: BTN.TRIANGLE, s: FACES.top[0], c: FACES.top[1] }, { x: 560, y: 163, b: BTN.CIRCLE, s: FACES.right[0], c: FACES.right[1] }, { x: 520, y: 204, b: BTN.CROSS, s: FACES.bottom[0], c: FACES.bottom[1] }, { x: 480, y: 163, b: BTN.SQUARE, s: FACES.left[0], c: FACES.left[1] }] as f}
      <circle cx={f.x} cy={f.y} r="18" class="btn" class:on={on(f.b)} />
      <text x={f.x} y={f.y + 6} class="face" fill={on(f.b) ? "#fff" : f.c}>{f.s}</text>
    {/each}

    <!-- sticks: grey dot = the stick, blue dot = what the game gets -->
    {#each [{ cx: 215, cy: 255, x: ax(raw?.lx), y: -ax(raw?.ly), o: ev?.outLeft, b: BTN.L3 }, { cx: 425, cy: 255, x: ax(raw?.rx), y: -ax(raw?.ry), o: ev?.outRight, b: BTN.R3 }] as s}
      <circle cx={s.cx} cy={s.cy} r={STICK_R + 8} class="stick-base" class:on={on(s.b)} />
      <circle cx={s.cx + s.x * STICK_R} cy={s.cy - s.y * STICK_R} r="9" class="stick-raw" />
      {#if s.o}
        <circle cx={s.cx + s.o[0] * STICK_R} cy={s.cy - s.o[1] * STICK_R} r="6" class="stick-out" />
      {/if}
    {/each}
  </svg>

  <div class="side">
    <div class="legend">
      <span><i class="dot raw"></i> Stick</span>
      <span><i class="dot out"></i> Lo que recibe el juego</span>
    </div>
    {#if raw && !xbox}
      <div class="sensor">
        <h3>Giroscopio</h3>
        {#each ["X", "Y", "Z"] as axis, i}
          <div class="bar-row">
            <span>{axis}</span>
            <div class="bar"><div class="mid" style={signed(raw.gyro[i], 8000)}></div></div>
          </div>
        {/each}
      </div>
      <div class="sensor">
        <h3>Acelerómetro</h3>
        {#each ["X", "Y", "Z"] as axis, i}
          <div class="bar-row">
            <span>{axis}</span>
            <div class="bar"><div class="mid" style={signed(raw.accel[i], 9000)}></div></div>
          </div>
        {/each}
      </div>
    {/if}
    {#if raw}
      <div class="sensor">
        <h3>Gatillos hacia el juego</h3>
        <div class="bar-row"><span>L2</span><div class="bar"><div class="fill" style="width: {(ev?.outL2 ?? 0) * 100}%"></div></div></div>
        <div class="bar-row"><span>R2</span><div class="bar"><div class="fill" style="width: {(ev?.outR2 ?? 0) * 100}%"></div></div></div>
      </div>
    {:else}
      <p class="muted">Esperando datos del control…</p>
    {/if}
  </div>
</div>

<style>
  .live {
    display: grid;
    grid-template-columns: 1fr 260px;
    gap: 20px;
    align-items: start;
  }
  .pad {
    width: 100%;
    max-height: 420px;
  }
  .slot {
    fill: #222a3b;
  }
  .fill {
    fill: var(--accent);
  }
  .lbl {
    fill: #cfd6e4;
    font-size: 12px;
    text-anchor: middle;
    font-weight: 600;
  }
  .face {
    font-size: 18px;
    text-anchor: middle;
    font-weight: 700;
  }
  .btn {
    fill: #262e42;
    stroke: #3a4560;
    stroke-width: 1.5;
    transition: fill 0.05s;
  }
  .btn.on {
    fill: var(--accent);
    stroke: #a5b8ff;
  }
  .touch {
    fill: #fff;
    opacity: 0.85;
  }
  .stick-base {
    fill: #141926;
    stroke: #3a4560;
    stroke-width: 2;
  }
  .stick-base.on {
    stroke: var(--accent);
    stroke-width: 4;
  }
  .stick-raw {
    fill: #8b95aa;
  }
  .stick-out {
    fill: var(--accent);
    stroke: #fff;
    stroke-width: 1.5;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .legend {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    margin-right: 6px;
    vertical-align: -1px;
  }
  .dot.raw {
    background: #8b95aa;
  }
  .dot.out {
    background: var(--accent);
  }
  .sensor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .bar-row {
    display: grid;
    grid-template-columns: 22px 1fr;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .bar {
    height: 8px;
    background: #222a3b;
    border-radius: 8px;
    position: relative;
    overflow: hidden;
  }
  .bar .fill {
    height: 100%;
    background: var(--accent);
  }
  /* centred bar for signed values */
  .bar .mid {
    position: absolute;
    top: 0;
    bottom: 0;
    background: var(--accent-2);
  }
</style>
