import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { demoBridge } from "./demo-state";
import type { AppSettings, AppState, HookDockBridge, HookResponse, HookSource, OperationResult, SourceProfile } from "./types";

const nativeBridge: HookDockBridge = {
  getState: () => invoke<AppState>("get_state"),
  onState: (callback) => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<AppState>("app-state", (event) => callback(event.payload)).then((nextUnlisten) => {
      if (disposed) nextUnlisten();
      else unlisten = nextUnlisten;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  },
  respond: (id: string, response: HookResponse) => invoke<OperationResult>("respond_to_hook", { id, response }),
  dismissEvent: (id: string) => invoke<OperationResult>("dismiss_event", { id }),
  focusEventTerminal: (id: string) => invoke<OperationResult>("focus_event_terminal", { id }),
  updateSettings: (patch: Partial<AppSettings>) => invoke<AppSettings>("update_settings", { patch }),
  updateSourceProfile: (id: HookSource, patch: Partial<Omit<SourceProfile, "id">>) => invoke<SourceProfile[]>("update_source_profile", { id, patch }),
  installHooks: (sources: HookSource[]) => invoke<OperationResult>("install_hooks", { sources }),
  uninstallHooks: (sources: HookSource[]) => invoke<OperationResult>("uninstall_hooks", { sources }),
  copyGenericCommand: () => invoke<OperationResult>("copy_generic_command"),
  sendTestEvent: () => invoke<OperationResult>("send_test_event"),
  openFolder: (path: string) => invoke<OperationResult>("open_folder", { path }),
  openUrl: (url: string) => invoke<OperationResult>("open_url", { url }),
  setCompact: (compact: boolean) => invoke<OperationResult>("set_compact", { compact }),
  snapWindow: () => invoke<OperationResult>("snap_window"),
  hideWindow: () => { void invoke("hide_window"); },
};

export const bridge = isTauri() ? nativeBridge : demoBridge;
