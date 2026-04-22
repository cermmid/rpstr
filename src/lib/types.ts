export type ModelProfile = "lite" | "standard" | "pro" | "cloud-only";

export interface Patient {
  id: string;
  pseudonym: string;
  createdAt: string;
}

export interface Icd10Code {
  code: string;
  labelPl: string;
  description?: string;
}

export interface VisitSummary {
  chiefComplaint: string;
  mentalStateExam: string;
  diagnosis: string;
  recommendations: string;
  medications: string;
  suggestedCodes: Icd10Code[];
}

export type VisitStatus =
  | "draft"
  | "recording"
  | "transcribing"
  | "summarizing"
  | "ready"
  | "saved";

export interface Visit {
  id: string;
  patientId: string;
  startedAt: string;
  status: VisitStatus;
  consentLogged: boolean;
  transcript?: string;
  summary?: VisitSummary;
  acceptedCodes?: string[];
  modelUsed?: string;
  tokensInput?: number;
  tokensOutput?: number;
  costPln?: number;
}

export type SummarizationBackend = "local-ollama" | "claude-byok";

export interface SystemProbe {
  freeDiskGb: number;
  totalRamGb: number;
  gpu: "cuda" | "directml" | "metal" | "cpu-only";
}

export interface AppSettings {
  backend: SummarizationBackend;
  profile: ModelProfile;
  modelsDir: string;
  audioRetentionDays: number;
  claudeModel?: "claude-haiku-4-5-20251001" | "claude-sonnet-4-6" | "claude-opus-4-7";
  monthlyBudgetPln?: number;
  batchModeEnabled?: boolean;
}
