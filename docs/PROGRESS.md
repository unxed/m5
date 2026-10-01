# Прогресс m5

## Сейчас

Последняя закрытая: **T-03** (2026-10-01) — Спайк ввода (--key-test mode реализован).
Следующая: **T-04** — Полная реализация Key/Mods/Event и декодер для legacyterm.

## История

- **T-03** ✔ Спайк D-03: реализован `m5 --key-test` в raw mode, печатает сырые байты и символы ESC-последовательности. На Unix использует прямое чтение stdin, на Windows используется crossterm::event. **ТРЕБУЕТ ИНТЕРАКТИВНОГО ТЕСТИРОВАНИЯ** перед продолжением (проверить, передаёт ли crossterm APC и неизвестные последовательности). Результат → обновить DECISIONS D-03.
- **T-02** ✔ `.github/workflows/ci.yml` с тестами (Linux/macOS/Windows), fmt, clippy, cargo-deny, Android cross-compile. `deny.toml` для проверки лицензий (BSD-3 compatible).
- **T-01** ✔ Workspace из 6 крейтов (m5-term, m5-ui, m5-config, m5-fs, m5-ops, m5); бинарь печатает версию; rust-toolchain.toml; docs/PROGRESS.md, DECISIONS.md, QUESTIONS.md.
