/// Tauri serializuje Rust errors jako stringi, nie obiekty Error. Standardowe
/// `e instanceof Error` nigdy nie jest true, więc fallback maskuje treść.
/// Ten helper zawsze zwraca pełen komunikat.
export function errMsg(e: unknown, fallback = "Wystąpił błąd."): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "toString" in e) return String(e);
  return fallback;
}
