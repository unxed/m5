# m5: Midnight Commander on Rust

Воссоздаём Midnight Commander на Rust.

## Reference

Берём вот эту версию:
https://github.com/unxed/mcommander/tree/staging

Поскольку оригинал под GPL, дословное переписывание запрещено, оригинальный код только референс по возможностям и UX, он же источник правды по ним.

## Features

- Режим Far UX - по умолчанию
- Двухпанельный файловый менеджер
- Поддержка keymap и скинов M-Commander

## Build Requirements

- **Rust**: stable (see `rust-toolchain.toml`)
- **MSRV** (Minimum Supported Rust Version): будет определена в T-01 (пока — latest stable)

## Building

```bash
cargo build --release
./target/release/m5
```

## Состояние

Проект в самом начале. Есть Cargo workspace из крейтов `m5`, `m5-config`, `m5-fs`, `m5-ops`, `m5-term`, `m5-ui` и CI (rustfmt, clippy, тесты на Linux/macOS/Windows, cargo-deny, кросс-сборка под Android). Написаны тип `InputEvent` (формат Win32/far2l), режим `--key-test` и декодеры ввода терминала в `m5-term` (модуль `decode`; какие протоколы уже разбираются — в `docs/PROGRESS.md`); `m5-ui`, `m5-config`, `m5-fs`, `m5-ops` — пустые заглушки, файлового менеджера пока нет. Интерактивная проверка на реальных терминалах не проводилась. Подробности — `docs/PROGRESS.md`.

## Development

Вытаскивать новые плюшки из апстрима агентами время от времени. И декларировать что мы приветствуем и обратное перетекание.

## Documentation

- `DESIGN.md` — полная архитектура и план итераций
- `docs/PROGRESS.md` — статус задач
- `docs/DECISIONS.md` — принятые архитектурные решения
- `docs/QUESTIONS.md` — открытые вопросы и замечания

