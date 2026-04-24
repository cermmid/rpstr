import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  ModelProfile,
  SetupProgress,
  SystemProbe,
  Visit,
  VisitSummary,
  WhisperSize,
} from "./types";

export const api = {
  unlock: (password: string) => invoke<boolean>("unlock_vault", { password }),

  probeSystem: () => invoke<SystemProbe>("probe_system"),
  downloadProfile: (profile: ModelProfile, modelsDir: string) =>
    invoke<void>("download_profile", { profile, modelsDir }),

  // Iteracja 2 — realny kreator.
  checkOllamaInstalled: () => invoke<boolean>("check_ollama_installed"),
  installOllama: () => invoke<void>("install_ollama"),
  pullOllamaModel: (name: string) =>
    invoke<void>("pull_ollama_model", { name }),
  downloadWhisperCpp: () =>
    invoke<{ binPath: string }>("download_whisper_cpp"),
  downloadWhisperModel: (size: WhisperSize) =>
    invoke<{ modelPath: string }>("download_whisper_model", { size }),
  finishSetup: (input: {
    whisperBin: string;
    whisperModel: string;
    ollamaModel: string;
  }) => invoke<void>("finish_setup", { input }),
  isSetupCompleted: () => invoke<boolean>("is_setup_completed"),

  onSetupProgress: (cb: (p: SetupProgress) => void): Promise<UnlistenFn> =>
    listen<SetupProgress>("setup:progress", (e) => cb(e.payload)),

  listVisits: () => invoke<Visit[]>("list_visits"),
  getVisit: (visitId: string) => invoke<Visit>("get_visit", { visitId }),
  createVisit: (patientPseudonym: string) =>
    invoke<Visit>("create_visit", { patientPseudonym }),
  logConsent: (visitId: string) => invoke<void>("log_consent", { visitId }),

  startRecording: () => invoke<void>("start_recording"),
  stopRecording: () => invoke<{ durationMs: number }>("stop_recording"),

  transcribe: (visitId: string) =>
    invoke<{ transcript: string }>("transcribe", { visitId }),

  summarize: (visitId: string, transcript: string) =>
    invoke<VisitSummary>("summarize", { visitId, transcript }),

  searchIcd10: (query: string) =>
    invoke<Array<{ code: string; labelPl: string }>>("search_icd10", { query }),

  saveVisit: (visitId: string, visit: Partial<Visit>) =>
    invoke<Visit>("save_visit", { visitId, patch: visit }),

  exportPdf: (visitId: string) => invoke<string>("export_pdf", { visitId }),

  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (patch: Partial<AppSettings>) =>
    invoke<AppSettings>("update_settings", { patch }),

  getCostStats: (month: string) =>
    invoke<{ visits: number; tokensInput: number; tokensOutput: number; totalPln: number }>(
      "get_cost_stats",
      { month },
    ),
};
