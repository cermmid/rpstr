# Regulamin korzystania z aplikacji rpstr (szkielet)

> **UWAGA**: Szkielet roboczy. Wymaga weryfikacji prawnej przed produkcją.

**Wersja**: 0.1 · **Data**: do uzupełnienia

## 1. Status aplikacji

Aplikacja rpstr jest **narzędziem roboczym** wspomagającym lekarza w
sporządzaniu notatek z wizyt. **Nie jest** oficjalnym systemem dokumentacji
medycznej (EDM). Oficjalna dokumentacja pozostaje po stronie lekarza (mMedica,
drEryk, gabinet.gov.pl itp.).

## 2. Odpowiedzialność

- Lekarz jako administrator danych odpowiada za treść dokumentacji wynikowej.
- Aplikacja generuje **szkice** na podstawie AI; każdy szkic wymaga manualnej
  weryfikacji przed użyciem.
- Producent aplikacji nie ponosi odpowiedzialności za błędy w podsumowaniach.

## 3. Dane pacjentów

- Aplikacja domyślnie działa w pełni lokalnie. Dane nie opuszczają urządzenia.
- Tryb BYOK Claude (opcjonalny, domyślnie wyłączony) wysyła zde-identyfikowany
  transkrypt do Anthropic (USA). Aktywacja wymaga świadomej akceptacji DPIA.

## 4. Bezpieczeństwo

- Baza szyfrowana AES-256 (SQLCipher).
- Hasło wymagane przy każdym uruchomieniu.
- **Utrata hasła = utrata dostępu**. Obowiązkowy backup klucza.

## 5. Aktualizacje

Auto-updater pobiera podpisane binarki. Aktualizacje mogą zmieniać model AI i
prompt — lekarz powinien zapoznać się z changelog'iem.

## 6. Kontakt

*Do uzupełnienia*.
