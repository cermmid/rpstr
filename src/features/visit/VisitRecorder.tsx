import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "@/lib/api";
import type { Icd10Code, VisitSummary } from "@/lib/types";
import { Icd10Picker } from "@/features/icd10/Icd10Picker";

type Phase =
  | "consent"
  | "idle"
  | "recording"
  | "transcribing"
  | "transcript"
  | "summarizing"
  | "review";

export function VisitRecorder() {
  const navigate = useNavigate();
  const [phase, setPhase] = useState<Phase>("consent");
  const [pseudonym, setPseudonym] = useState("");
  const [consent, setConsent] = useState(false);
  const [visitId, setVisitId] = useState<string | null>(null);
  const [transcript, setTranscript] = useState("");
  const [summary, setSummary] = useState<VisitSummary | null>(null);
  const [acceptedCodes, setAcceptedCodes] = useState<Icd10Code[]>([]);
  const [err, setErr] = useState<string | null>(null);

  async function startVisit() {
    if (!consent || !pseudonym.trim()) return;
    setErr(null);
    try {
      const v = await api.createVisit(pseudonym.trim());
      await api.logConsent(v.id);
      setVisitId(v.id);
      setPhase("idle");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Nie udało się utworzyć wizyty.");
    }
  }

  async function startRec() {
    setErr(null);
    try {
      await api.startRecording();
      setPhase("recording");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Błąd mikrofonu.");
    }
  }

  async function stopRec() {
    if (!visitId) return;
    setPhase("transcribing");
    try {
      await api.stopRecording();
      const { transcript: t } = await api.transcribe(visitId);
      setTranscript(t);
      setPhase("transcript");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Błąd transkrypcji.");
      setPhase("idle");
    }
  }

  async function runSummary() {
    if (!visitId) return;
    setPhase("summarizing");
    try {
      const s = await api.summarize(visitId, transcript);
      setSummary(s);
      setAcceptedCodes(s.suggestedCodes);
      setPhase("review");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Błąd podsumowania.");
      setPhase("transcript");
    }
  }

  async function save() {
    if (!visitId || !summary) return;
    try {
      await api.saveVisit(visitId, {
        transcript,
        summary,
        acceptedCodes: acceptedCodes.map((c) => c.code),
      });
      navigate("/app/visits");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Błąd zapisu.");
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <h1 className="text-2xl font-semibold">
        {phase === "consent" ? "Nowa wizyta" : "Wizyta w toku"}
      </h1>

      {err && (
        <div className="card border-red-300 bg-red-50 text-sm text-red-700 dark:bg-red-950 dark:border-red-900">
          {err}
        </div>
      )}

      {phase === "consent" && (
        <div className="card space-y-4">
          <label className="block">
            <span className="text-sm font-medium">Pseudonim pacjenta</span>
            <input
              className="input mt-1"
              placeholder="np. JK-1987"
              value={pseudonym}
              onChange={(e) => setPseudonym(e.target.value)}
            />
            <span className="mt-1 block text-xs text-slate-500">
              Bez PESEL-u i pełnego nazwiska. To tylko Twój wewnętrzny identyfikator.
            </span>
          </label>
          <label className="flex items-start gap-3 rounded-md bg-amber-50 p-3 dark:bg-amber-950">
            <input
              type="checkbox"
              className="mt-1"
              checked={consent}
              onChange={(e) => setConsent(e.target.checked)}
            />
            <span className="text-sm">
              Pacjent wyraził zgodę na nagrywanie rozmowy wyłącznie w celu
              sporządzenia roboczego podsumowania. Nagranie zostanie usunięte po
              transkrypcji.
            </span>
          </label>
          <button
            className="btn-primary"
            disabled={!consent || !pseudonym.trim()}
            onClick={startVisit}
          >
            Rozpocznij
          </button>
        </div>
      )}

      {phase === "idle" && (
        <div className="card space-y-4">
          <p className="text-sm text-slate-600 dark:text-slate-400">
            Gotowe do nagrywania. Audio trafia tylko do pamięci RAM, nie jest
            zapisywane na dysku.
          </p>
          <button className="btn-primary" onClick={startRec}>
            ● Nagrywaj
          </button>
        </div>
      )}

      {phase === "recording" && (
        <div className="card space-y-4">
          <div className="flex items-center gap-3">
            <span className="inline-block h-3 w-3 animate-pulse rounded-full bg-red-600" />
            <span className="font-medium">Nagrywanie…</span>
          </div>
          <button className="btn-danger" onClick={stopRec}>
            ■ Stop i transkrybuj
          </button>
        </div>
      )}

      {phase === "transcribing" && (
        <div className="card text-sm text-slate-600">
          Whisper pracuje lokalnie… (5-60 s w zależności od sprzętu)
        </div>
      )}

      {phase === "transcript" && (
        <div className="card space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="font-medium">Transkrypt (edytowalny)</h2>
            <button className="btn-primary" onClick={runSummary}>
              Podsumuj →
            </button>
          </div>
          <textarea
            className="input min-h-[300px] font-mono text-sm"
            value={transcript}
            onChange={(e) => setTranscript(e.target.value)}
          />
        </div>
      )}

      {phase === "summarizing" && (
        <div className="card text-sm text-slate-600">
          LLM przetwarza transkrypt…
        </div>
      )}

      {phase === "review" && summary && (
        <ReviewPanel
          summary={summary}
          acceptedCodes={acceptedCodes}
          visitId={visitId}
          onSummaryChange={setSummary}
          onCodesChange={setAcceptedCodes}
          onSave={save}
        />
      )}
    </div>
  );
}

function ReviewPanel(props: {
  summary: VisitSummary;
  acceptedCodes: Icd10Code[];
  visitId: string | null;
  onSummaryChange: (s: VisitSummary) => void;
  onCodesChange: (c: Icd10Code[]) => void;
  onSave: () => void;
}) {
  const { summary, acceptedCodes, onSummaryChange, onCodesChange, onSave } = props;
  const navigate = useNavigate();
  const set = (key: keyof VisitSummary, value: string) =>
    onSummaryChange({ ...summary, [key]: value });

  return (
    <div className="space-y-4">
      <div className="card space-y-3">
        <div className="mb-2 inline-block rounded bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-800 dark:bg-amber-900 dark:text-amber-100">
          szkic AI — zweryfikuj przed zapisem
        </div>
        <Field label="Wywiad / Chief complaint" value={summary.chiefComplaint} onChange={(v) => set("chiefComplaint", v)} />
        <Field label="Stan psychiczny" value={summary.mentalStateExam} onChange={(v) => set("mentalStateExam", v)} />
        <Field label="Rozpoznanie" value={summary.diagnosis} onChange={(v) => set("diagnosis", v)} />
        <Field label="Zalecenia" value={summary.recommendations} onChange={(v) => set("recommendations", v)} />
        <Field label="Leki" value={summary.medications} onChange={(v) => set("medications", v)} />
      </div>

      <div className="card space-y-3">
        <h2 className="font-medium">Kody ICD-10</h2>
        <Icd10Picker selected={acceptedCodes} onChange={onCodesChange} />
      </div>

      <div className="flex flex-wrap gap-2">
        <button className="btn-primary" onClick={onSave}>
          Zapisz do dokumentacji
        </button>
        <button className="btn-secondary" onClick={() => navigator.clipboard?.writeText(renderText(summary, acceptedCodes))}>
          Kopiuj do schowka
        </button>
        {props.visitId && (
          <button
            className="btn-secondary"
            onClick={() => navigate(`/app/visits/${props.visitId}/print`)}
          >
            Eksport PDF
          </button>
        )}
      </div>
    </div>
  );
}

function Field(props: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <label className="block">
      <span className="text-sm font-medium">{props.label}</span>
      <textarea
        className="input mt-1 min-h-[80px]"
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
      />
    </label>
  );
}

function renderText(s: VisitSummary, codes: Icd10Code[]): string {
  return [
    `WYWIAD: ${s.chiefComplaint}`,
    `STAN PSYCHICZNY: ${s.mentalStateExam}`,
    `ROZPOZNANIE: ${s.diagnosis}`,
    `ZALECENIA: ${s.recommendations}`,
    `LEKI: ${s.medications}`,
    `KODY ICD-10: ${codes.map((c) => `${c.code} ${c.labelPl}`).join("; ")}`,
    "",
    "(szkic AI — zweryfikuj)",
  ].join("\n");
}
