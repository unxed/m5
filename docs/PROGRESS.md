# Прогресс m5

Статус отражает только то, что подтверждено кодом в репозитории и прогонами CI.

## Сейчас

CI на main был красным начиная с коммита T-03 (fmt, clippy, test, deny, Android — все падали).
Починено в PR #1 (fmt, clippy, test x3 ОС, deny licenses, Android cross-build с NDK-линкером — зелёные). Cargo.lock закоммичен (бинарный workspace), тесты идут с `--locked`.

Следующая задача: **T-04** — декодеры ввода на базе `InputEvent` (Rust, crates/m5-term). **Не сделана.**

## История

- **T-03** (частично). Реализовано: тип `InputEvent`, `ControlKeyState`, `EventType`, константы VK,
  юнит-тесты типов (m5-term/src/key.rs), режим `m5 --key-test` (m5-term/src/key_test.rs).
  Не сделано: проверка на реальных терминалах (интерактивно), проверка crossterm на APC/неизвестные
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
