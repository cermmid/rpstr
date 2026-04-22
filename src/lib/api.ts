import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ModelProfile,
  SystemProbe,
  Visit,
  VisitSummary,
} from "./types";

export const api = {
  unlock: (password: string) => invoke<boolean>("unlock_vault", { password }),

  probeSystem: () => invoke<SystemProbe>("probe_system"),
  downloadProfile: (profile: ModelProfile, modelsDir: string) =>
    invoke<void>("download_profile", { profile, modelsDir }),

  listVisits: () => invoke<Visit[]>("list_visits"),
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
