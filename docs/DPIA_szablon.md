# DPIA — szkielet

> **UWAGA**: To jest szkielet roboczy, nie kompletny dokument prawny. Przed
> produkcją musi go uzupełnić i zweryfikować IOD albo prawnik specjalizujący
> się w RODO medycznym.

## 1. Opis operacji przetwarzania

**Administrator**: lekarz prowadzący praktykę (indywidualny użytkownik aplikacji).
**Aplikacja**: rpstr — lokalny asystent psychiatry.

### Cel przetwarzania
Sporządzenie roboczego szkicu dokumentacji medycznej na podstawie nagrania
rozmowy lekarz-pacjent, z sugerowanymi kodami ICD-10.

### Kategorie danych
- **Dane szczególnej kategorii (art. 9 RODO)**: dane dotyczące zdrowia psychicznego.
- Nie przetwarzamy: PESEL, adresu, pełnego imienia i nazwiska (pseudonim wybierany
  przez lekarza).

### Etapy przetwarzania
1. Nagrywanie (bufor RAM, bez zapisu na dysku).
2. Transkrypcja (whisper.cpp lokalnie).
3. Podsumowanie LLM (lokalne Ollama **albo** Claude API w trybie BYOK).
4. Akceptacja lekarza i zapis do zaszyfrowanej bazy (SQLCipher AES-256).
5. Eksport do oficjalnego EDM (poza zakresem aplikacji).

## 2. Konieczność i proporcjonalność

Aplikacja zmniejsza obciążenie dokumentacyjne lekarza, skracając czas sporządzania
szkicu. Alternatywy: ręczne notatki (czasochłonne), dyktafon + ręczna transkrypcja
(mniej dokładne), EDM z wbudowaną transkrypcją chmurową (transfer do USA bez
pseudonimizacji).

## 3. Ocena ryzyka

| Ryzyko | Prawdopodobieństwo | Waga | Mitygacja |
|---|---|---|---|
| Wyciek bazy lokalnej | średnie | wysokie | SQLCipher AES-256, hasło PBKDF2 600k, klucz w OS keychain |
| Utrata hasła → brak dostępu | niskie | średnie | Kreator backupu klucza przy pierwszym uruchomieniu |
| Błędna transkrypcja/halucynacja LLM | wysokie | średnie | Manualna akceptacja przed zapisem, znak wodny "szkic AI" |
| Transfer do USA (tryb BYOK Claude) | wysokie przy włączonym BYOK | wysokie | OFF domyślnie, pseudonimizacja NER przed wysyłką, SCC/DPA Anthropic |
| Nieautoryzowany dostęp do laptopa | średnie | wysokie | Hasło przy starcie, auto-lock po X minutach (TODO) |

## 4. Zgoda pacjenta

Aplikacja wymusza check-box "Pacjent wyraził zgodę na nagrywanie" przed każdym
uruchomieniem nagrywania. Zgoda jest logowana w `consent_log` z timestamp i
wersją regulaminu. Lekarz zobowiązany jest poinformować pacjenta ustnie o:
- celu nagrywania (sporządzenie szkicu dokumentacji),
- fakcie automatycznego usunięcia audio po transkrypcji,
- prawie do odmowy (odmowa nie wpływa na jakość opieki medycznej),
- prawie do żądania usunięcia notatek.

## 5. Konsultacja z IOD

*Do uzupełnienia*: data i wynik konsultacji z Inspektorem Ochrony Danych.

## 6. Decyzja

*Do uzupełnienia*: czy DPIA wymagała uprzedniej konsultacji z PUODO (art. 36 RODO).
