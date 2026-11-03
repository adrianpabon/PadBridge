// Mirrors src-tauri/src/settings.rs, device.rs and ds/protocol.rs (serde camelCase).

export type Charging = "discharging" | "charging" | "full" | "error";

export interface ControllerInfo {
  id: string;
  kind: "dualsense" | "xbox";
  name: string;
  connection: "usb" | "bluetooth" | "wireless";
  battery: number | null;
  charging: Charging;
  slot: number | null;
  emulating: boolean;
  emulationError: string | null;
}

export interface TouchPoint {
  active: boolean;
  id: number;
  x: number;
  y: number;
}

export interface InputState {
  lx: number;
  ly: number;
  rx: number;
  ry: number;
  l2: number;
  r2: number;
  buttons: number;
  touch: [TouchPoint, TouchPoint];
  gyro: [number, number, number];
  accel: [number, number, number];
  battery: number;
  charging: Charging;
}

export interface InputEvent {
  id: string;
  raw: InputState;
  outLeft: [number, number];
  outRight: [number, number];
  outL2: number;
  outR2: number;
}

export interface AppPrefs {
  closeToTray: boolean;
  startMinimized: boolean;
}

export interface Overview {
  vigemInstalled: boolean;
  conflicts: string[];
  controllers: ControllerInfo[];
  prefs: AppPrefs;
  settingsPath: string;
}

export type LightbarMode = "static" | "rainbow" | "battery" | "off";
export type PlayerLedMode = "auto" | "custom" | "off";
export type MicLed = "off" | "on" | "pulse";

export type TriggerEffect =
  | { mode: "off" }
  | { mode: "resistance"; start: number; strength: number }
  | { mode: "slope"; start: number; end: number; startStrength: number; endStrength: number }
  | { mode: "weapon"; start: number; end: number; strength: number }
  | { mode: "vibration"; start: number; amplitude: number; frequency: number }
  | { mode: "bow"; start: number; end: number; strength: number; snap: number }
  | { mode: "galloping"; start: number; end: number; firstFoot: number; secondFoot: number; frequency: number }
  | { mode: "machine"; start: number; end: number; ampA: number; ampB: number; frequency: number; period: number }
  | { mode: "zones"; strengths: number[] };

export type TriggerMode = TriggerEffect["mode"];

export interface StickSettings {
  sensitivity: number;
  deadzone: number;
  antiDeadzone: number;
  curve: number;
  invertX: boolean;
  invertY: boolean;
}

export interface TriggerRange {
  deadzone: number;
  max: number;
}

export interface Profile {
  emulate: boolean;
  lightbar: { mode: LightbarMode; color: [number, number, number]; brightness: number; speed: number };
  playerLeds: { mode: PlayerLedMode; custom: number; brightness: number };
  micLed: MicLed;
  leftTrigger: TriggerEffect;
  rightTrigger: TriggerEffect;
  rumble: { enabled: boolean; intensity: number };
  leftStick: StickSettings;
  rightStick: StickSettings;
  l2: TriggerRange;
  r2: TriggerRange;
  touchpad: { mouse: boolean; speed: number; clickAsBack: boolean };
}

/** Button bits, as in ds/protocol.rs `btn`. */
export const BTN = {
  CROSS: 1 << 0,
  CIRCLE: 1 << 1,
  SQUARE: 1 << 2,
  TRIANGLE: 1 << 3,
  L1: 1 << 4,
  R1: 1 << 5,
  L2: 1 << 6,
  R2: 1 << 7,
  CREATE: 1 << 8,
  OPTIONS: 1 << 9,
  L3: 1 << 10,
  R3: 1 << 11,
  PS: 1 << 12,
  TOUCHPAD: 1 << 13,
  MUTE: 1 << 14,
  UP: 1 << 15,
  DOWN: 1 << 16,
  LEFT: 1 << 17,
  RIGHT: 1 << 18,
  FN_L: 1 << 19,
  FN_R: 1 << 20,
  PADDLE_L: 1 << 21,
  PADDLE_R: 1 << 22,
} as const;
