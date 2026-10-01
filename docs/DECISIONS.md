# Принятые решения (Decisions)

## D-01: Cargo workspace из нескольких крейтов

**Решение**: Использовать workspace из отдельных крейтов (m5-term, m5-ui, m5-config, m5-fs, m5-ops, m5).

**Почему**: Изоляция, быстрые тесты, явные границы между компонентами.

**Альтернативы**: Монолитный крейт. Отклонено из соображений модульности.

**Где в коде**: Cargo.toml (workspace), структура crates/.

---

## D-02: crossterm — только для raw mode, размера терминала, вывода, и ввода на Windows

**Решение**: crossterm используется минимально: raw mode, размер терминала, буферизированный вывод, ввод только на Windows.

**Почему**: На Unix нужна точная контроль над последовательностями (far2l, kitty, win32-input-mode). crossterm не гарантирует пробросить неизвестные последовательности.

**Где в коде**: m5-term/src/ (будущие модули input, output).

---

## D-03: Win32-совместимый формат InputEvent для всех событий

**Решение**: Используем Win32 INPUT_RECORD-совместимый формат (структура из unxed/winkeys) для всех событий.
Вместо собственного Key/Mods, используем универсальную `InputEvent` с полями:
- `EventType` (Key, Mouse, Focus, Paste, Far2l, Resize)
- `ControlKeyState` (Win32 флаги: LEFT_ALT, RIGHT_ALT, LEFT_CTRL, RIGHT_CTRL, SHIFT, ENHANCED, NUM_LOCK, CAPS_LOCK, SCROLL_LOCK)
- `VirtualKeyCode`/`VirtualScanCode` (Win32 коды клавиш)
- `Char`/`UnshiftedChar` (символы)
- `KeyDown`/`RepeatCount` (статус и повторы)
- Плюс типоспецифичные поля (MouseX/Y, Far2lCommand, и т.д.)

**Почему**: Стабильный и проверенный формат, используемый far2l, f4, и всеми крупными клиентами.
Идеально конвертится в/из kitty protocol. Упрощает интеграцию и совместимость с существующей инфраструктурой.

**Статус**: тип `InputEvent` и константы определены в m5-term/src/key.rs. Декодеров (T-04..T-06) пока нет;
`m5 --key-test` строит `InputEvent` только для одиночных печатных ASCII-байтов, остальное печатает как сырые байты.
Решение принято, но на реальных терминалах не проверено: D-03 подтверждается/пересматривается после T-04..T-06.
Замечание о reference: unxed/winkeys — Go-библиотека; здесь значения констант сверены с ней и с far2l WinPort/WinCompat.h.
Поля `Paste`/`Far2l`/`Resize` EventType взяты из winkeys и не являются значениями Win32.

**Альтернативы**: Собственный KeyCode enum + Mods бит-флаг (отклонено: менее гибко, не совместимо с far2l/f4).

**Где в коде**: m5-term/src/key.rs (InputEvent, ControlKeyState, EventType, вспомогательные типы),
m5-term/src/key_test.rs (спайк T-03), m5-term/src/lib.rs (публичный экспорт типов).

---

## D-04: Собственный двойной cell-буфер + diff-рендер

**Решение**: Собственный буфер вместо ratatui (TUI-framework).

**Почему**: mc-подобные диалоги с тенями, точный контроль над фокусом, цветом, рамками. ratatui избыточен и навязывает свою архитектуру.

**Где в коде**: m5-ui/src/ (Buffer, diff_render, виджеты).

---

## D-05: Однопоточный UI-цикл + рабочие потоки для файловых операций

**Решение**: Главный цикл (`poll_event` → render → flush) в одном потоке; операции (copy, delete, …) в отдельных потоках; коммуникация через std::sync::mpsc.

**Почему**: Простота, без async-рантайма, понятная логика синхронизации.

**Где в коде**: m5/src/main.rs (главный цикл), m5-ops/src/ (spawn worker threads).

---

## D-06: Все команды — строковые action names в стиле mc

**Решение**: Действия хранятся в keymap как строковые имена (`"Copy"`, `"CdParent"`, …), не как enum.

**Почему**: keymap.ini работает как есть; будущий Lua-API вызывает те же действия через строковое имя.

**Где в коде**: m5-config/src/ (Keymap model), m5/src/ (ActionRegistry).

---

## D-07: Far-режим = оверлей поверх стандартного keymap

**Решение**: `assets/keymap.far.ini` — дополнительный слой, который перекрывает клавиши для selected actions.

**Почему**: Так устроено в M-Commander; Far-режим включается настройкой (по умолчанию `true` в m5, отличие от оригинала).

**Где в коде**: m5-config/src/keymap.rs (слои keymap), assets/keymap.far.ini.

---

## D-08: Каталог настроек: свой `m5`, с чтением `mc6` как fallback

**Решение**: Настройки пишутся в `~/.config/m5/` (Unix) или `%APPDATA%\m5` (Windows); читаются туда же, но fallback на `~/.config/mc6/` если файл не найден.

**Почему**: Не портить конфиг M-Commander; подхватывать его скины/keymap для совместимости. Настройка `compat_mc6_dirs` (по умолчанию `true`).

**Где в коде**: m5-config/src/paths.rs, m5-config/src/config.rs.

---

## D-09: Rust edition 2024, stable toolchain, MSRV фиксируется в T-01

**Решение**: Используем edition 2024, stable channel. MSRV (minimal supported Rust version) определяется в T-01 и фиксируется в rust-toolchain.toml и README.

**Почему**: Стабильность, поддержка новых фич, явная документация требований.

**Где в коде**: rust-toolchain.toml, README.md (раздел Requirements), Cargo.toml (edition).

---

## D-10: Без `unsafe`, кроме платформенных обёрток в `m5-term`/`m5-fs`

**Решение**: Только платформенный code с `unsafe` в модулях для Unix-specific (SIGWINCH, raw mode) и Windows-specific. Все unsafe помечаются комментариями `// SAFETY:`.

**Почему**: Безопасность, читаемость, при необходимости лёгкий аудит.

**Где в коде**: m5-term/src/ (ввод/выход), m5-fs/src/ (filesystem API).

---

---

## D-11: Декодеры ввода — `Decoder` выдаёт `InputEvent` напрямую

**Решение**: `m5_term::Decoder` — чистый конечный автомат без терминала и часов: `feed(&[u8], &mut Vec<InputEvent>)`
и `flush_timeout(&mut Vec<InputEvent>)` (вызывается, когда после ESC в течение таймаута ничего не пришло).
Отдельных типов `Key`/`Mods`/`Event` из DESIGN §5.1 нет (ответ на Q-12): keymap работает на `InputEvent` напрямую.
Каждый протокол — отдельный подмодуль `decode/`, общие таблицы — `decode/keys.rs`.

**Соглашения легаси-протоколов** (xterm, SS3, символы):
- события только нажатия: `key_down = true`, `is_legacy = true`, `repeat_count = 1`, scan-код 0;
- Ctrl+буква даёт `char_code` = управляющий символ (как у консоли Windows) и `LEFT_CTRL_PRESSED`;
  0x08 и 0x7F — `VK_BACK`; `Enter`/`Tab`/`Esc` — свои VK и символы;
- VK печатных ASCII-символов берутся по раскладке US; заглавная буква и «верхние» знаки дают `SHIFT_PRESSED`;
  для остальных символов (кириллица и т.д.) VK = 0;
- стрелки, Home/End, PgUp/PgDn, Ins/Del получают `ENHANCED_KEY` (как в far2l);
- модификаторы xterm: Shift, Alt, Ctrl — в левые флаги Win32; Meta/Super не имеют флага Win32 и отбрасываются;
- `ESC` + символ — Alt+символ; `ESC ESC [ ...` — Alt поверх последовательности; одиночный `ESC` — клавиша Esc после таймаута;
- ответы терминала, не являющиеся вводом (позиция курсора, DA, DSR), и неизвестные CSI молча отбрасываются;
  `CSI 1;m R` считается F3 с модификатором (неоднозначно с отчётом о позиции курсора для строки 1);
- строки APC (`ESC _`) потребляются целиком; `ESC P`/`ESC ]` строками не считаются, чтобы не съедать Alt+P и Alt+].

**Почему**: чистая функция тестируется без терминала; формат решён в D-03.

**Где в коде**: m5-term/src/decode/.
