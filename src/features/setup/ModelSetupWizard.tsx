import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";
import type { SetupProgress, SystemProbe, WhisperSize } from "@/lib/types";

/// Iteracja 2: kreator pobiera wszystko do działającego MVP (Ollama + Bielik 7B
/// + whisper.cpp + ggml-medium.bin).  Profile Lite/Standard/Pro z iteracji 1 są
/// wykonane — w iteracji 3 (cargo bundle) wrócą jako wybór jakości.
///
/// Tag Bielika przez HuggingFace: Ollama 0.21+ wspiera `hf.co/<org>/<repo>:<quant>`.
/// Jeśli pull się wywali (stara Ollama albo rate-limit HF), wizard przełącza się
/// na `qwen2.5:3b` (oficjalna biblioteka Ollamy, zawsze dostępny).

const DEFAULT_OLLAMA_MODEL = "hf.co/speakleash/Bielik-7B-Instruct-v0.1-GGUF:Q4_K_M";
const FALLBACK_OLLAMA_MODEL = "qwen2.5:3b";
const DEFAULT_WHISPER: WhisperSize = "medium";

type Stage =
  | "probe"
  | "confirm"
  | "ollama-install"
  | "ollama-pull"
  | "whisper-cpp"
  | "whisper-model"
  | "done"
  | "error";

export function ModelSetupWizard() {
  const navigate = useNavigate();
  const [stage, setStage] = useState<Stage>("probe");
  const [probe, setProbe] = useState<SystemProbe | null>(null);
  const [progress, setProgress] = useState<SetupProgress | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [usedFallback, setUsedFallback] = useState(false);
  const progressUnlisten = useRef<(() => void) | null>(null);

  useEffect(() => {
    api.probeSystem().then(setProbe).catch((e) => setErr(errMsg(e)));
    api.onSetupProgress(setProgress).then((un) => {
      progressUnlisten.current = un;
    });
    return () => {
      progressUnlisten.current?.();
    };
  }, []);

  async function runSetup() {
    setErr(null);
    try {
      // 1. Ollama instalacja (jeśli potrzeba).
      if (probe && !probe.ollamaInstalled) {
        setStage("ollama-install");
        await api.installOllama();
      }

      // 2. Pull Bielika.  Jeśli się wywali, spróbuj fallbacku na qwen.
      setStage("ollama-pull");
      let ollamaModel = DEFAULT_OLLAMA_MODEL;
      try {
        await api.pullOllamaModel(DEFAULT_OLLAMA_MODEL);
      } catch (e) {
        // Stara Ollama bez `hf.co/…` albo rate-limit HuggingFace.
        console.warn("[rpstr/setup] Bielik pull failed, fallback to qwen:", e);
        setUsedFallback(true);
        ollamaModel = FALLBACK_OLLAMA_MODEL;
        await api.pullOllamaModel(FALLBACK_OLLAMA_MODEL);
      }

      // 3. whisper.cpp binarka.
      setStage("whisper-cpp");
      const { binPath } = await api.downloadWhisperCpp();

      // 4. ggml-medium.bin.
      setStage("whisper-model");
      const { modelPath } = await api.downloadWhisperModel(DEFAULT_WHISPER);

      // 5. Zapis do settings + oznacz setup_completed.
      await api.finishSetup({
        whisperBin: binPath,
        whisperModel: modelPath,
        ollamaModel,
      });
      setStage("done");
    } catch (e) {
      setErr(errMsg(e, "Błąd podczas konfiguracji."));
      setStage("error");
    }
  }

  if (stage === "probe" && probe) {
    // Automatycznie przechodzimy do confirm po załadowaniu probe.
    setStage("confirm");
  }

  return (
    <div className="mx-auto max-w-3xl p-6 space-y-4">
      <h1 className="text-2xl font-semibold">Konfiguracja</h1>

      {!probe && (
        <div className="text-sm text-slate-500">Sprawdzam sprzęt…</div>
      )}

      {probe && stage === "confirm" && (
        <ConfirmStep
          probe={probe}
          onStart={runSetup}
        />
      )}

      {(stage === "ollama-install" ||
        stage === "ollama-pull" ||
        stage === "whisper-cpp" ||
        stage === "whisper-model") && (
        <ProgressStep stage={stage} progress={progress} fallback={usedFallback} />
      )}

      {stage === "done" && (
        <div className="card space-y-3">
          <div className="text-lg font-medium">Gotowe!</div>
          <p className="text-sm text-slate-600 dark:text-slate-400">
            Wszystko pobrane i skonfigurowane. Możesz zaczynać pierwszą wizytę.
            {usedFallback && (
              <>
                <br />
                <span className="text-amber-700 dark:text-amber-400">
                  Uwaga: zamiast Bielika 7B używany jest qwen2.5:3b (Bielik nie
                  był dostępny przez twoją wersję Ollamy). Możesz to zmienić w
                  Ustawieniach po aktualizacji Ollamy.
                </span>
              </>
            )}
          </p>
          <button
            className="btn-primary"
            onClick={() => navigate("/app/visits", { replace: true })}
          >
            Przejdź do wizyt
          </button>
        </div>
      )}

      {err && (
        <div className="card border-red-300 bg-red-50 dark:bg-red-950 dark:border-red-900 text-sm">
          <div className="font-medium text-red-900 dark:text-red-200">Błąd</div>
          <div className="text-red-800 dark:text-red-300">{err}</div>
          <button className="btn-secondary mt-2" onClick={runSetup}>
            Spróbuj ponownie
          </button>
        </div>
      )}
    </div>
  );
}

function ConfirmStep({
  probe,
  onStart,
}: {
  probe: SystemProbe;
  onStart: () => void;
}) {
  const needOllama = !probe.ollamaInstalled;
  const lowDisk = probe.freeDiskGb < 8;

  return (
    <div className="space-y-4">
      <div className="card text-sm space-y-1">
        <div>
          <strong>{probe.freeDiskGb.toFixed(1)} GB</strong> wolnego miejsca ·{" "}
          <strong>{probe.totalRamGb} GB</strong> RAM · GPU:{" "}
          <strong>{probe.gpu}</strong>
        </div>
        <div className="text-slate-500">
          Ollama: {probe.ollamaInstalled ? "zainstalowana" : "do zainstalowania"}
        </div>
      </div>

      <div className="card space-y-3">
        <h2 className="font-medium">Co zostanie pobrane (~6.6 GB)</h2>
        <ul className="text-sm space-y-1 list-disc pl-5">
          {needOllama && <li>Ollama (runtime LLM) — ~700 MB</li>}
          <li>Bielik 7B (polski model medyczny, Q4_K_M) — ~4.4 GB</li>
          <li>whisper.cpp (transkrypcja) — ~20 MB</li>
          <li>Whisper Medium (model transkrypcji PL) — ~1.5 GB</li>
        </ul>
        <p className="text-xs text-slate-500">
          Pobieranie wymaga stabilnego internetu (~15-30 min). Po setupie
          wszystko działa offline.
        </p>
        {needOllama && (
          <p className="text-xs text-amber-700 dark:text-amber-400">
            Za chwilę Windows zapyta o zgodę administratora dla instalacji
            Ollamy — to normalne.
          </p>
        )}
        {lowDisk && (
          <p className="text-xs text-red-700 dark:text-red-400">
            Masz mało wolnego miejsca ({probe.freeDiskGb.toFixed(1)} GB). Zwolnij
            co najmniej 8 GB przed rozpoczęciem.
          </p>
        )}
      </div>

      <button className="btn-primary" onClick={onStart} disabled={lowDisk}>
        Rozpocznij pobieranie
      </button>
    </div>
  );
}

const STAGE_LABELS: Record<Stage, string> = {
  probe: "Probe",
  confirm: "Potwierdzenie",
  "ollama-install": "Instalacja Ollamy",
  "ollama-pull": "Pobieranie modelu LLM",
  "whisper-cpp": "Pobieranie whisper.cpp",
  "whisper-model": "Pobieranie modelu Whisper",
  done: "Gotowe",
  error: "Błąd",
};

function ProgressStep({
  stage,
  progress,
  fallback,
}: {
  stage: Stage;
  progress: SetupProgress | null;
  fallback: boolean;
}) {
  const label = STAGE_LABELS[stage];
  const pct = progress?.stage === stage.replace("confirm", "") ? progress.percent : 0;

  return (
    <div className="card space-y-3">
      <div className="flex items-baseline justify-between">
        <div className="font-medium">{label}</div>
        <div className="text-xs text-slate-500">
          {progress ? `${progress.percent.toFixed(0)}%` : ""}
        </div>
      </div>
      <div className="h-2 w-full rounded bg-slate-200 dark:bg-slate-800 overflow-hidden">
        <div
          className="h-2 bg-brand-500 transition-all"
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="text-xs text-slate-500 truncate">
        {progress?.message ?? "…"}
      </div>
      {fallback && stage === "ollama-pull" && (
        <div className="text-xs text-amber-700 dark:text-amber-400">
          Pierwsza próba (Bielik 7B) nieudana — używam fallbacku qwen2.5:3b.
        </div>
      )}
    </div>
  );
}
