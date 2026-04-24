# rpstr — instrukcja dla testera

Dziękujemy za testy! Poniżej wszystko, co musisz wiedzieć, żeby uruchomić
aplikację i zgłosić uwagi.

## Co to jest?

**rpstr** to desktopowe narzędzie robocze dla psychiatrów — nagrywa wizytę,
lokalnie ją transkrybuje (Whisper) i generuje szkic dokumentacji (SOAP +
sugerowane kody ICD-10) lokalnym modelem LLM (Bielik 7B). **Dane pacjentów
nie opuszczają Twojego laptopa.**

To **nie jest** oficjalna dokumentacja medyczna. Generowany SOAP i kody to
**szkic do weryfikacji** — wklej go do swojego EDM (mMedica, drEryk,
gabinet.gov.pl) po ręcznej korekcie.

## Wymagania

- Windows 10 / 11 (64-bit)
- ~8 GB wolnego miejsca na dysku
- Minimum 8 GB RAM (zalecane 16 GB)
- Mikrofon
- Stabilny internet **na pierwsze uruchomienie** (~30 min pobierania); potem
  apka działa offline

## Instalacja

1. Pobierz plik `rpstr_0.1.0_x64_pl-PL.msi` z maila.
2. Kliknij 2x. Windows prawdopodobnie pokaże ostrzeżenie
   **"Windows protected your PC"** — to dlatego, że apka jest jeszcze bez
   certyfikatu EV code-signing (testowa wersja).
3. Kliknij **"More info"** (lub "Więcej informacji") → **"Run anyway"**
   (lub "Uruchom mimo to").
4. Pozwól instalatorowi się skończyć.

## Pierwsze uruchomienie

1. Odpal **rpstr** z menu Start.
2. Ustaw hasło (odblokowuje szyfrowaną lokalną bazę). Zapamiętaj je! **Reset
   hasła nie jest obecnie obsługiwany** — utrata hasła = utrata wpisów.
3. Aplikacja pokaże ekran konfiguracji. Zaakceptuj pobieranie ~6.6 GB. Windows
   zapyta raz o zgodę administratora (to instalacja Ollamy — runtime'u LLM).
4. Czekaj ~15-30 min. Postęp jest widoczny.
5. Po zakończeniu przejdziesz do listy wizyt.

## Jak używać

1. Kliknij **"Nowa wizyta"**.
2. Wpisz pseudonim pacjenta (np. "JK-1987", **nie PESEL**).
3. Zaznacz checkbox zgody na nagrywanie (musi być zaznaczony, inaczej nie
   można nagrywać).
4. Kliknij **Nagrywaj**. Mów / prowadź wizytę. Kliknij **Stop**.
5. Kliknij **Transkrybuj**. Zajmie to ~30 s na CPU dla 10-minutowego nagrania.
6. Kliknij **Podsumuj**. Bielik 7B wygeneruje SOAP (~30-60 s na GTX 960M,
   ~2 min na pure CPU).
7. **Przeczytaj, popraw, zaakceptuj kody ICD-10.**
8. Kliknij **Zapisz**. Audio domyślnie zostaje skasowane (ustawienia → retencja).
9. **Kopiuj do schowka** → wklej do EDM. Albo **Eksportuj PDF**.

## Na co zwrócić uwagę (co raportować)

- **Jakość SOAP**: czy sekcje Wywiad / Stan psychiczny / Rozpoznanie /
  Zalecenia / Leki są sensowne? Halucynacje? Pominięte ważne rzeczy?
- **Trafność ICD-10**: czy 1-3 zaproponowane kody są adekwatne?
- **Transkrypcja**: końcówki polskie, nazwy leków, ChAD/PTSD/lęk uogólniony —
  czy są prawidłowe?
- **Prędkość**: ile minut zajął Ci cały flow (nagranie 15 min → zapis)?
- **UX**: gdzie się zawahałeś? Co było nieoczywiste? Co przeszkadzało?
- **Crash'e**: jeśli apka się wywala, zanotuj co robiłeś. Logi są w
  `%APPDATA%\rpstr\logs\` (ale nie są jeszcze zbierane automatycznie — podaj
  opisowo).

## Jak zgłosić feedback

Maila / telefon — masz nasz kontakt. Zapisz też ekran albo nagraj krótki
filmik jak coś dziwnego się dzieje.

## Jeśli coś nie działa

### Pobieranie się wiesza
- Zrestartuj apkę, kreator zacznie od miejsca gdzie skończył (oprócz modelu
  Ollamy — ten zaczyna od zera, bo Ollama sama zarządza cache'em).

### Po nagraniu widzisz "transkrypt niedostępny"
- Znaczy, że setup nie zakończył się do końca. Zajrzyj w **Ustawienia → Model
  Whisper** i kliknij "Medium (1.5 GB)" żeby dopobrać.

### "Ollama: connection refused"
- Otwórz PowerShell, wpisz `ollama serve`. Nie zamykaj okna. Wróć do rpstr.
  (Docelowo rpstr sam uruchomi Ollamę — to w następnej iteracji.)

### Bielik 7B pobiera się w nieskończoność
- Jeśli Twoja Ollama jest starsza niż 0.21, nie obsługuje tagów
  `hf.co/…`. Kreator automatycznie przełączy się na qwen2.5:3b (słabszy, ale
  działa). Jeśli chcesz ręcznie: `ollama pull qwen2.5:3b`.

## Bezpieczeństwo

- Wszystkie dane w `%APPDATA%\rpstr\db.sqlite` są szyfrowane AES-256 (SQLCipher).
- Klucz główny w Windows Credential Manager — nigdy na dysku.
- **Brak komunikacji sieciowej** poza pobraniami w setupie (i auto-updaterem
  w przyszłości). Możesz po setupie odłączyć laptop od sieci — wizyty działają.
- Tryb "Claude BYOK" (ustawienia → backend) wysyła zde-identyfikowany
  transkrypt do Anthropic (USA) — **domyślnie wyłączony**, włączany świadomie.

## Kontakt

Jeśli utknąłeś — napisz. Przygotowaliśmy Cię na ~2h testów, ale w razie
czego dzwoń. Dziękujemy!
