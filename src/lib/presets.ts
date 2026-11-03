import type { TriggerEffect, TriggerMode } from "./types";

export const MODE_LABELS: Record<TriggerMode, string> = {
  off: "Apagado",
  resistance: "Resistencia",
  slope: "Resistencia progresiva",
  weapon: "Arma (gatillo con clic)",
  vibration: "Vibración",
  bow: "Arco",
  galloping: "Galope",
  machine: "Ametralladora",
  zones: "Por zonas (personalizado)",
};

export const MODE_HELP: Record<TriggerMode, string> = {
  off: "El gatillo se siente normal.",
  resistance: "Cuesta lo mismo apretar desde el punto de inicio hasta el fondo.",
  slope: "La resistencia sube (o baja) a lo largo del recorrido.",
  weapon: "Resistencia que cede de golpe, como el disparador de un arma.",
  vibration: "El gatillo vibra a partir del punto de inicio.",
  bow: "Se tensa como la cuerda de un arco y al final empuja de vuelta.",
  galloping: "Golpes rítmicos, como un caballo al galope.",
  machine: "Vibración que alterna dos fuerzas, como una ametralladora.",
  zones: "Elige la fuerza de cada una de las 10 zonas del recorrido.",
};

/** Sensible defaults for each mode, used when the mode is picked. */
export function defaultEffect(mode: TriggerMode): TriggerEffect {
  switch (mode) {
    case "off":
      return { mode };
    case "resistance":
      return { mode, start: 2, strength: 5 };
    case "slope":
      return { mode, start: 0, end: 9, startStrength: 1, endStrength: 8 };
    case "weapon":
      return { mode, start: 3, end: 6, strength: 7 };
    case "vibration":
      return { mode, start: 0, amplitude: 6, frequency: 30 };
    case "bow":
      return { mode, start: 1, end: 6, strength: 6, snap: 6 };
    case "galloping":
      return { mode, start: 0, end: 9, firstFoot: 2, secondFoot: 5, frequency: 8 };
    case "machine":
      return { mode, start: 1, end: 9, ampA: 5, ampB: 2, frequency: 12, period: 4 };
    case "zones":
      return { mode, strengths: [0, 0, 1, 2, 3, 4, 5, 6, 7, 8] };
  }
}

export interface Preset {
  name: string;
  left: TriggerEffect;
  right: TriggerEffect;
}

export const PRESETS: Preset[] = [
  { name: "Normal", left: { mode: "off" }, right: { mode: "off" } },
  {
    name: "Rifle",
    left: { mode: "resistance", start: 2, strength: 3 },
    right: { mode: "weapon", start: 3, end: 6, strength: 7 },
  },
  {
    name: "Pistola",
    left: { mode: "resistance", start: 2, strength: 3 },
    right: { mode: "weapon", start: 2, end: 4, strength: 8 },
  },
  {
    name: "Escopeta",
    left: { mode: "resistance", start: 2, strength: 4 },
    right: { mode: "weapon", start: 4, end: 7, strength: 8 },
  },
  {
    name: "Ametralladora",
    left: { mode: "resistance", start: 2, strength: 3 },
    right: { mode: "machine", start: 1, end: 9, ampA: 6, ampB: 2, frequency: 14, period: 3 },
  },
  {
    name: "Arco",
    left: { mode: "off" },
    right: { mode: "bow", start: 1, end: 6, strength: 7, snap: 7 },
  },
  {
    name: "Carreras",
    left: { mode: "slope", start: 0, end: 9, startStrength: 2, endStrength: 7 },
    right: { mode: "slope", start: 0, end: 9, startStrength: 1, endStrength: 5 },
  },
  {
    name: "Caballo",
    left: { mode: "off" },
    right: { mode: "galloping", start: 0, end: 9, firstFoot: 2, secondFoot: 5, frequency: 8 },
  },
  {
    name: "Motor",
    left: { mode: "vibration", start: 0, amplitude: 3, frequency: 25 },
    right: { mode: "vibration", start: 2, amplitude: 6, frequency: 40 },
  },
  {
    name: "Máxima resistencia",
    left: { mode: "resistance", start: 0, strength: 8 },
    right: { mode: "resistance", start: 0, strength: 8 },
  },
];
