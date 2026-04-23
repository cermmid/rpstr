import { FormEvent, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "@/lib/api";
import { errMsg } from "@/lib/err";

export function UnlockView() {
  const navigate = useNavigate();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    setBusy(true);
    try {
      const ok = await api.unlock(password);
      if (!ok) {
        setErr("Nieprawidłowe hasło.");
        return;
      }
      navigate("/app/visits", { replace: true });
    } catch (e) {
      setErr(errMsg(e, "Nie udało się odblokować bazy."));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex h-full items-center justify-center p-6">
      <form onSubmit={onSubmit} className="card w-full max-w-sm space-y-4">
        <div>
          <h1 className="text-xl font-semibold">rpstr</h1>
          <p className="text-sm text-slate-500">
            Wpisz hasło, aby odblokować dokumentację.
          </p>
        </div>
        <input
          type="password"
          className="input"
          placeholder="Hasło"
          autoFocus
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
        />
        {err && <div className="text-sm text-red-600">{err}</div>}
        <button type="submit" className="btn-primary w-full" disabled={busy}>
          {busy ? "Odblokowywanie…" : "Odblokuj"}
        </button>
        <p className="text-xs text-slate-500">
          Baza szyfrowana lokalnie (AES-256). Jeśli zapomnisz hasła, odzyskanie
          nie będzie możliwe — przywróć z kopii zapasowej klucza.
        </p>
      </form>
    </div>
  );
}
