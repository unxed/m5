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
- **MSRV** (Minimum Supported Rust Version): 1.85 (`rust-version` в `Cargo.toml`, edition 2024). Конструкции, появившиеся в более новых версиях (например, `let`-цепочки `if a && let …`, стабильные с 1.88), использовать нельзя; CI-джоб `msrv` собирает и тестирует проект ровно на 1.85.0.

## Building

```bash
cargo build --release
./target/release/m5
```

## Состояние

Проект в самом начале. Есть Cargo workspace из крейтов `m5`, `m5-config`, `m5-fs`, `m5-ops`, `m5-term`, `m5-ui` и CI (rustfmt, clippy, тесты на Linux/macOS/Windows, cargo-deny, кросс-сборка под Android). Реально написан только тип `InputEvent` (формат Win32/far2l) и режим `--key-test` в `m5-term`; `m5-ui`, `m5-config`, `m5-fs`, `m5-ops` — пустые заглушки, файлового менеджера пока нет. Декодеры ввода (T-04) не сделаны; интерактивная проверка T-03 на терминалах не проводилась. Подробности — `docs/PROGRESS.md`.

## Development

Вытаскивать новые плюшки из апстрима агентами время от времени. И декларировать что мы приветствуем и обратное перетекание.

## Documentation

- `DESIGN.md` — полная архитектура и план итераций
- `docs/PROGRESS.md` — статус задач
- `docs/DECISIONS.md` — принятые архитектурные решения
- `docs/QUESTIONS.md` — открытые вопросы и замечания

