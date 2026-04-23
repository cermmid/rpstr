import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";
import type { AppSettings } from "@/lib/types";
import { CostDashboard } from "./CostDashboard";

export function SettingsView() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => setErr(errMsg(e)));
  }, []);

  async function update(patch: Partial<AppSettings>) {
    try {
      const next = await api.updateSettings(patch);
      setSettings(next);
    } catch (e) {
      setErr(errMsg(e));
    }
  }

  if (!settings) return <div className="text-sm text-slate-500">Wczytywanie ustawień…</div>;

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <h1 className="text-2xl font-semibold">Ustawienia</h1>
      {err && <div className="text-sm text-red-600">{err}</div>}

      <section className="card space-y-3">
        <h2 className="font-medium">Model</h2>
        <label className="block">
          <span className="text-sm">Profil modeli</span>
          <select
            className="input mt-1"
            value={settings.profile}
            onChange={(e) => update({ profile: e.target.value as AppSettings["profile"] })}
          >
            <option value="lite">Lite (~2.3 GB) — Whisper Small + Qwen 3B</option>
            <option value="standard">Standard (~5 GB) — Whisper Medium + Llama 3.1 8B</option>
            <option value="pro">Pro (~8.5 GB) — Whisper Large + Bielik 11B</option>
            <option value="cloud-only">Cloud-only (BYOK Claude) — tylko lokalny STT</option>
          </select>
        </label>
        <label className="block">
          <span className="text-sm">Backend podsumowań</span>
          <select
            className="input mt-1"
            value={settings.backend}
            onChange={(e) => update({ backend: e.target.value as AppSettings["backend"] })}
          >
            <option value="local-ollama">Lokalny (Ollama) — dane zostają na laptopie</option>
            <option value="claude-byok">Claude API (BYOK) — transfer do USA, wymaga DPIA</option>
          </select>
        </label>
      </section>

      {settings.backend === "claude-byok" && (
        <section className="card space-y-3 border-amber-300 bg-amber-50 dark:bg-amber-950 dark:border-amber-900">
          <h2 className="font-medium">BYOK Claude — konfiguracja</h2>
          <p className="text-xs text-amber-800 dark:text-amber-200">
            Włączenie tego trybu oznacza, że zde-identyfikowany transkrypt zostanie
            wysłany do Anthropic (USA). Przed użyciem zapoznaj się z DPIA i
            skonsultuj z IOD. Klucz API trzymany jest w systemowym keychain.
          </p>
          <label className="block">
            <span className="text-sm">Model</span>
            <select
              className="input mt-1"
              value={settings.claudeModel ?? "claude-haiku-4-5-20251001"}
              onChange={(e) => update({ claudeModel: e.target.value as AppSettings["claudeModel"] })}
            >
              <option value="claude-haiku-4-5-20251001">Haiku 4.5 (zalecany — najtańszy)</option>
              <option value="claude-sonnet-4-6">Sonnet 4.6 (trudniejsze przypadki)</option>
              <option value="claude-opus-4-7">Opus 4.7 (niepotrzebnie drogi dla SOAP)</option>
            </select>
          </label>
          <label className="block">
            <span className="text-sm">Miesięczny limit budżetu (PLN)</span>
            <input
              type="number"
              min={0}
              className="input mt-1"
              value={settings.monthlyBudgetPln ?? 50}
              onChange={(e) => update({ monthlyBudgetPln: Number(e.target.value) })}
            />
            <span className="mt-1 block text-xs text-slate-500">
              Po przekroczeniu apka automatycznie przełącza się na model lokalny.
            </span>
          </label>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={settings.batchModeEnabled ?? false}
              onChange={(e) => update({ batchModeEnabled: e.target.checked })}
            />
            <span className="text-sm">
              Tryb nocny (Batch API, -50%) — podsumowania przygotowywane wsadowo do rana
            </span>
          </label>
        </section>
      )}

      <section className="card space-y-3">
        <h2 className="font-medium">Retencja danych</h2>
        <label className="block">
          <span className="text-sm">Dni przechowywania audio (0 = usuń natychmiast)</span>
          <input
            type="number"
            min={0}
            max={90}
            className="input mt-1"
            value={settings.audioRetentionDays}
            onChange={(e) => update({ audioRetentionDays: Number(e.target.value) })}
          />
        </label>
      </section>

      <CostDashboard />
    </div>
  );
}
