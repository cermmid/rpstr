import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { Icd10Code } from "@/lib/types";

export function Icd10Picker(props: {
  selected: Icd10Code[];
  onChange: (codes: Icd10Code[]) => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Icd10Code[]>([]);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    if (query.trim().length < 2) {
      setResults([]);
      return;
    }
    let alive = true;
    setSearching(true);
    const t = setTimeout(async () => {
      try {
        const r = await api.searchIcd10(query.trim());
        if (alive) setResults(r);
      } finally {
        if (alive) setSearching(false);
      }
    }, 200);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [query]);

  function toggle(c: Icd10Code) {
    const has = props.selected.some((x) => x.code === c.code);
    props.onChange(
      has ? props.selected.filter((x) => x.code !== c.code) : [...props.selected, c],
    );
  }

  return (
    <div className="space-y-3">
      {props.selected.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {props.selected.map((c) => (
            <button
              key={c.code}
              onClick={() => toggle(c)}
              className="inline-flex items-center gap-1 rounded-full bg-brand-600 px-3 py-1 text-xs text-white hover:bg-brand-700"
              title="Kliknij, żeby usunąć"
            >
              <span className="font-mono">{c.code}</span>
              <span>{c.labelPl}</span>
              <span>×</span>
            </button>
          ))}
        </div>
      )}

      <input
        className="input"
        placeholder="Szukaj w bazie ICD-10 (np. „depresja”)"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />

      {searching && <div className="text-xs text-slate-500">Szukam…</div>}

      {results.length > 0 && (
        <ul className="max-h-56 overflow-auto rounded-md border border-slate-200 dark:border-slate-800">
          {results.map((c) => {
            const picked = props.selected.some((x) => x.code === c.code);
            return (
              <li key={c.code}>
                <button
                  onClick={() => toggle(c)}
                  className={`flex w-full items-start gap-3 px-3 py-2 text-left text-sm hover:bg-slate-100 dark:hover:bg-slate-800 ${
                    picked ? "bg-brand-50 dark:bg-slate-800" : ""
                  }`}
                >
                  <span className="min-w-[60px] font-mono text-brand-700">{c.code}</span>
                  <span>{c.labelPl}</span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
