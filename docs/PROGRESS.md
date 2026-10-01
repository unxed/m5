# Прогресс m5

Статус отражает только то, что подтверждено кодом в репозитории и прогонами CI.

## Сейчас

CI на main был красным начиная с коммита T-03 (fmt, clippy, test, deny, Android — все падали).
Починено в PR #1 (fmt, clippy, test x3 ОС, deny licenses, Android cross-build с NDK-линкером — зелёные). Cargo.lock закоммичен (бинарный workspace), тесты идут с `--locked`.

Идёт **T-04..T-06** — декодеры ввода на базе `InputEvent` (Rust, crates/m5-term/src/decode), по одному протоколу на PR.
Разобрано (код и юнит-тесты в репозитории; тесты запускаются только в CI): обычные символы UTF-8 и управляющие байты,
Alt как префикс ESC, клавиши xterm/VT (CSI и SS3 с модификаторами, F1..F20, навигация, keypad, modifyOtherKeys,
консоль Linux, rxvt); протокол клавиатуры kitty (разбор `CSI u` и форм `CSI ... ~`/буква с типом события,
обратное преобразование `InputEvent` -> kitty по флагам: `decode::kitty::encode`); режим win32-input-mode (`CSI Vk;Sc;Uc;Kd;Cs;Rc _`, разбор и `decode::win32::encode`). расширения far2l (APC `f2l`: клавиши K/k/C/c, мышь M/m, размер S; ответы и подтверждение — событиями `Far2l`). Остальное — мышь SGR, bracketed paste, фокус — ещё не сделано.
Интерактивная проверка `m5 --key-test` в xterm, kitty и WezTerm проведена в CI (`.github/workflows/key-test.yml`), результаты — в `docs/T-03-KEY-TEST.md`; far2l APC и win32-input-mode до приложения не проверены.

## История

- **T-06** (шаг 4, far2l APC). Модуль `decode::far2l`; `InputEvent` получил поля размера (`term_width`/`term_height`, `resize_to`). Решения — D-14.
- **T-05** (шаг 3, win32-input-mode). Модуль `decode::win32`; в `Decoder` появилось состояние между последовательностями (`State`). Решения — D-13.
- **T-08** (шаг 1, буфер). `m5-ui/src/buffer.rs`: `Color`, `Attrs`, `Style`, `Rect`, `Cell`, `Buffer` (`put_str` с графемами и шириной, `fill`, `to_text`, `to_style_map`) и юнит-тесты; тесты запускаются только в CI. Не сделано из T-08: `frame`, `shadow`, `diff_render`, деградация цветов 16/256/truecolor.
- **T-05** (шаг 2, kitty). Модуль `decode::kitty`: разбор и обратное кодирование. Решения — D-12.
- **T-04** (в работе, шаг 1). `Decoder` (`feed` / `flush_timeout`) и разбор легаси-клавиш xterm. Решения — D-11.
- **T-03** (частично). Реализовано: тип `InputEvent`, `ControlKeyState`, `EventType`, константы VK,
  юнит-тесты типов (m5-term/src/key.rs), режим `m5 --key-test` (m5-term/src/key_test.rs).
  Проверка на xterm/kitty/WezTerm сделана (docs/T-03-KEY-TEST.md). Не сделано: far2l и win32-input-mode на терминале, который их реально шлёт, проверка crossterm на APC/неизвестные
  последовательности (требовалась DESIGN §9 T-03), показ результата владельцу. Ранее заявленное
  "валидация пройдена, 35+ тестов проходят" было неверно: в момент заявления CI был красным
  и тесты не компилировались; соответствующие документы (T-03-VALIDATION*, T-03-ISSUE-COMMENT) удалены.
- **D-03** ревизия: внутренний формат — Win32 InputEvent (см. DECISIONS.md).
- **T-02** CI (`.github/workflows/ci.yml`: test на Linux/macOS/Windows, fmt, clippy, cargo-deny licenses,
  Android cross-build) и `deny.toml`. Исходная версия не работала (deny.toml в устаревшем формате).
- **T-01** Workspace из 6 крейтов (m5-term, m5-ui, m5-config, m5-fs, m5-ops, m5), бинарь печатает версию,
  rust-toolchain.toml (stable), docs/*. Крейты m5-ui/m5-config/m5-fs/m5-ops пока пустые заглушки.

## Не сделано

T-04 и далее (декодеры xterm/kitty/win32-input/far2l, UnixTerminal, буфер, панели и т.д.) — см. DESIGN §9.
