import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";
import type { Visit } from "@/lib/types";

export function VisitList() {
  const [visits, setVisits] = useState<Visit[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const list = await api.listVisits();
        if (alive) setVisits(list);
      } catch (e) {
        if (alive) setErr(errMsg(e, "Błąd wczytywania."));
      } finally {
        if (alive) setLoading(false);
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Wizyty</h1>
        <Link to="/app/visits/new" className="btn-primary">
          + Nowa wizyta
        </Link>
      </div>

      {loading && <div className="text-sm text-slate-500">Wczytywanie…</div>}
      {err && <div className="text-sm text-red-600">{err}</div>}

      {!loading && visits.length === 0 && (
        <div className="card text-center text-sm text-slate-500">
          Brak zapisanych wizyt. Zacznij od „+ Nowa wizyta”.
        </div>
      )}

      <ul className="space-y-2">
        {visits.map((v) => (
          <li key={v.id}>
            <Link
              to={`/app/visits/${v.id}`}
              className="card flex items-center justify-between hover:border-brand-500"
            >
              <div>
                <div className="font-medium">{v.patientId}</div>
                <div className="text-xs text-slate-500">
                  {new Date(v.startedAt).toLocaleString("pl-PL")} · {statusLabel(v.status)}
                </div>
              </div>
              {v.acceptedCodes && v.acceptedCodes.length > 0 && (
                <div className="flex gap-1">
                  {v.acceptedCodes.slice(0, 3).map((c) => (
                    <span
                      key={c}
                      className="rounded bg-brand-50 px-2 py-0.5 text-xs font-mono text-brand-700"
                    >
                      {c}
                    </span>
                  ))}
                </div>
              )}
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}

function statusLabel(s: Visit["status"]): string {
  const map: Record<Visit["status"], string> = {
    draft: "szkic",
    recording: "nagrywanie",
    transcribing: "transkrypcja",
    summarizing: "podsumowanie",
    ready: "gotowe",
    saved: "zapisane",
  };
  return map[s];
}
