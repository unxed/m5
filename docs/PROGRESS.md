# Прогресс m5

## Сейчас

Последняя закрытая: **T-03-revised** (2026-10-01) — Спайк ввода на Win32 InputEvent (--key-test обновлен).
Следующая: **T-04** — Декодер для legacy xterm + kitty + far2l (на базе InputEvent).

## История

- **T-03-revised** ✔ Архитектурное решение: перейти на Win32-совместимый формат InputEvent (как в far2l/f4).
  - Реализован InputEvent в m5-term/src/key.rs с полной поддержкой всех типов событий (Key, Mouse, Focus, Paste, Far2l, Resize)
  - Обновлен `m5 --key-test` для вывода в формате InputEvent (сырые байты + Win32-стиль события)
  - Обновлена D-03 в DECISIONS.md с новым решением
  - Синтаксис совместим с kitty protocol и легко расширяется для far2l APC
  
- **T-03** ✔ (предыдущее) Спайк D-03: реализован `m5 --key-test` в raw mode.

- **T-02** ✔ `.github/workflows/ci.yml` с тестами (Linux/macOS/Windows), fmt, clippy, cargo-deny, Android cross-compile. `deny.toml` для проверки лицензий (BSD-3 compatible).

- **T-01** ✔ Workspace из 6 крейтов (m5-term, m5-ui, m5-config, m5-fs, m5-ops, m5); бинарь печатает версию; rust-toolchain.toml; docs/PROGRESS.md, DECISIONS.md, QUESTIONS.md.
