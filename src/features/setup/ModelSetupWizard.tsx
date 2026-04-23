import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";
import type { ModelProfile, SystemProbe } from "@/lib/types";

const PROFILES: Array<{
  id: ModelProfile;
  name: string;
  sizeGb: number;
  desc: string;
}> = [
  { id: "lite", name: "Lite", sizeGb: 2.3, desc: "Whisper Small + Qwen 2.5 3B. Dla starszych laptopów." },
  { id: "standard", name: "Standard", sizeGb: 5, desc: "Whisper Medium + Llama 3.1 8B. Rekomendowane." },
  { id: "pro", name: "Pro", sizeGb: 8.5, desc: "Whisper Large + Bielik 11B. Dla nowych laptopów z GPU." },
  { id: "cloud-only", name: "Cloud-only (BYOK)", sizeGb: 0.5, desc: "Whisper Small + Claude API. Wymaga akceptacji DPIA." },
];

export function ModelSetupWizard() {
  const navigate = useNavigate();
  const [probe, setProbe] = useState<SystemProbe | null>(null);
  const [chosen, setChosen] = useState<ModelProfile | null>(null);
  const [modelsDir, setModelsDir] = useState("");
  const [downloading, setDownloading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    api.probeSystem().then(setProbe).catch((e) => setErr(errMsg(e)));
  }, []);

  const recommended: ModelProfile = !probe
    ? "standard"
    : probe.freeDiskGb > 10 && probe.totalRamGb >= 16
      ? probe.freeDiskGb > 20 && probe.gpu !== "cpu-only"
        ? "pro"
        : "standard"
      : probe.freeDiskGb > 3
        ? "lite"
        : "cloud-only";

  async function start() {
    if (!chosen) return;
    setDownloading(true);
    setErr(null);
    try {
      await api.downloadProfile(chosen, modelsDir);
      navigate("/app/visits", { replace: true });
    } catch (e) {
      setErr(errMsg(e, "Błąd pobierania modeli."));
      setDownloading(false);
    }
  }

  return (
    <div className="mx-auto max-w-3xl p-6 space-y-4">
      <h1 className="text-2xl font-semibold">Konfiguracja modeli</h1>

      {probe ? (
        <div className="card text-sm">
          <div>
            <strong>{probe.freeDiskGb.toFixed(1)} GB</strong> wolnego miejsca ·{" "}
            <strong>{probe.totalRamGb} GB</strong> RAM · GPU:{" "}
            <strong>{probe.gpu}</strong>
          </div>
          <div className="mt-1 text-slate-500">
            Rekomendowany profil: <strong>{PROFILES.find((p) => p.id === recommended)?.name}</strong>
          </div>
        </div>
      ) : (
        <div className="text-sm text-slate-500">Sprawdzam sprzęt…</div>
      )}

      <div className="grid gap-2">
        {PROFILES.map((p) => {
          const notEnough = probe && p.sizeGb > probe.freeDiskGb;
          return (
            <button
              key={p.id}
              disabled={!!notEnough}
              onClick={() => setChosen(p.id)}
              className={`card text-left transition ${
                chosen === p.id ? "border-brand-500 ring-2 ring-brand-500" : ""
              } ${notEnough ? "opacity-50 cursor-not-allowed" : "hover:border-brand-500"}`}
            >
              <div className="flex items-baseline justify-between">
                <div className="font-medium">
                  {p.name} {p.id === recommended && <span className="text-xs text-brand-600">(rekomendowany)</span>}
                </div>
                <div className="text-xs text-slate-500">{p.sizeGb} GB</div>
              </div>
              <div className="text-sm text-slate-600 dark:text-slate-400">{p.desc}</div>
              {notEnough && (
                <div className="mt-1 text-xs text-red-600">
                  Za mało miejsca — wybierz inny dysk albo mniejszy profil.
                </div>
              )}
            </button>
          );
        })}
      </div>

      <label className="block">
        <span className="text-sm font-medium">Lokalizacja modeli</span>
        <input
          className="input mt-1"
          placeholder="%APPDATA%/rpstr/models (domyślnie)"
          value={modelsDir}
          onChange={(e) => setModelsDir(e.target.value)}
        />
        <span className="mt-1 block text-xs text-slate-500">
          Możesz wskazać inny dysk (np. D:\rpstr-models), jeśli systemowy jest pełny.
        </span>
      </label>

      {err && <div className="text-sm text-red-600">{err}</div>}

      <button className="btn-primary" disabled={!chosen || downloading} onClick={start}>
        {downloading ? "Pobieranie…" : "Pobierz i uruchom"}
      </button>

      <p className="text-xs text-slate-500">
        Pobieranie możesz wstrzymać i wznowić. Pliki weryfikowane są przez SHA-256.
      </p>
    </div>
  );
}
