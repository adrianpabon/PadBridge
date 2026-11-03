import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppPrefs, ControllerInfo, InputEvent, Overview, Profile } from "./types";

export const api = {
  overview: () => invoke<Overview>("get_overview"),
  profile: (id: string) => invoke<Profile>("get_profile", { id }),
  setProfile: (id: string, profile: Profile) => invoke<void>("set_profile", { id, profile }),
  resetProfile: (id: string) => invoke<Profile>("reset_profile", { id }),
  applyToAll: (id: string) => invoke<void>("apply_profile_to_all", { id }),
  watchInput: (id: string | null) => invoke<void>("watch_input", { id }),
  testRumble: (id: string, large: number, small: number) => invoke<void>("test_rumble", { id, large, small }),
  identify: (id: string) => invoke<void>("identify", { id }),
  setPrefs: (prefs: AppPrefs) => invoke<void>("set_prefs", { prefs }),
};

export const events = {
  controllers: (f: (c: ControllerInfo[]) => void): Promise<UnlistenFn> =>
    listen<ControllerInfo[]>("controllers", (e) => f(e.payload)),
  overview: (f: (o: Overview) => void): Promise<UnlistenFn> => listen<Overview>("overview", (e) => f(e.payload)),
  input: (f: (i: InputEvent) => void): Promise<UnlistenFn> => listen<InputEvent>("input", (e) => f(e.payload)),
};
