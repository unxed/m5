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

## Development

Вытаскивать новые плюшки из апстрима агентами время от времени. И декларировать что мы приветствуем и обратное перетекание.

## Documentation

- `DESIGN.md` — полная архитектура и план итераций
- `docs/PROGRESS.md` — статус задач
- `docs/DECISIONS.md` — принятые архитектурные решения
- `docs/QUESTIONS.md` — открытые вопросы и замечания

