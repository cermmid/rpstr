import { useEffect, useState, type ReactNode } from "react";
import { Navigate, useLocation } from "react-router-dom";
import { api } from "@/lib/api";

/// Blokuje trasy `/app/*` dopóki `setup_completed != true`.  Logika jest
/// oddzielona od `UnlockView`, bo unlock (otwarcie SQLCipher) i setup
/// (pobranie modeli) to niezależne rzeczy — tester mógłby odblokować bazę
/// z poprzedniej sesji, ale brakowałoby mu jeszcze modeli.
export function SetupGate({ children }: { children: ReactNode }) {
  const [state, setState] = useState<"loading" | "ok" | "missing">("loading");
  const loc = useLocation();

  useEffect(() => {
    let cancelled = false;
    api
      .isSetupCompleted()
      .then((done) => {
        if (cancelled) return;
        setState(done ? "ok" : "missing");
      })
      .catch(() => {
        if (!cancelled) setState("missing");
      });
    return () => {
      cancelled = true;
    };
  }, [loc.pathname]);

  if (state === "loading") {
    return <div className="p-6 text-sm text-slate-500">Wczytywanie…</div>;
  }
  if (state === "missing") {
    return <Navigate to="/setup" replace />;
  }
  return <>{children}</>;
}
