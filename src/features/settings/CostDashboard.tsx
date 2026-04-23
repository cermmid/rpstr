import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";

interface Stats {
  visits: number;
  tokensInput: number;
  tokensOutput: number;
  totalPln: number;
}

export function CostDashboard() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [month, setMonth] = useState(() => new Date().toISOString().slice(0, 7));
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .getCostStats(month)
      .then((s) => alive && setStats(s))
      .catch((e) => alive && setErr(errMsg(e)));
    return () => {
      alive = false;
    };
  }, [month]);

  return (
    <section className="card space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="font-medium">Koszty Claude API (tryb BYOK)</h2>
        <input
          type="month"
          className="input w-40"
          value={month}
          onChange={(e) => setMonth(e.target.value)}
        />
      </div>

      {err && <div className="text-sm text-red-600">{err}</div>}

      {stats ? (
        <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
          <Stat label="Wizyty" value={stats.visits.toString()} />
          <Stat label="Tokeny input" value={stats.tokensInput.toLocaleString("pl-PL")} />
          <Stat label="Tokeny output" value={stats.tokensOutput.toLocaleString("pl-PL")} />
          <Stat
            label="Razem"
            value={`${stats.totalPln.toFixed(2)} zł`}
            emphasis
          />
        </div>
      ) : (
        <div className="text-sm text-slate-500">Wczytywanie statystyk…</div>
      )}
      <p className="text-xs text-slate-500">
        W trybie w pełni lokalnym wszystkie wartości są zerowe — koszty powstają tylko
        przy włączonym backendzie Claude BYOK.
      </p>
    </section>
  );
}

function Stat(props: { label: string; value: string; emphasis?: boolean }) {
  return (
    <div
      className={`rounded-md border p-3 ${
        props.emphasis
          ? "border-brand-500 bg-brand-50 dark:bg-slate-800"
          : "border-slate-200 dark:border-slate-800"
      }`}
    >
      <div className="text-xs uppercase text-slate-500">{props.label}</div>
      <div className="text-lg font-semibold">{props.value}</div>
    </div>
  );
}
