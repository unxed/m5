# m5 — Design Document

> Midnight Commander (вариант M-Commander) на Rust, Far UX по умолчанию.
> Документ рассчитан на исполнителя-модель (Claude Haiku, extra high effort),
> которая берёт задачи по одной, строго по порядку итераций.
> Статус: v0.1, 2026-10-01. Покрывает MVP целиком и эскизно — дальнейшие итерации.

---

## 0. Как пользоваться этим документом (читать ПЕРВЫМ)

1. Открой `docs/PROGRESS.md` — там последняя закрытая задача. Бери **следующую** задачу из §9.
2. Одна сессия = одна задача `T-xx` (если задача мелкая — максимум две подряд из одной итерации).
3. Перед кодом перечитай: §1 (правила), раздел архитектуры для затрагиваемого крейта, критерии приёмки задачи.
4. Результат задачи = код + тесты + обновлённые `docs/PROGRESS.md` и (если были) `docs/QUESTIONS.md` / `docs/DECISIONS.md`.
5. Если в задаче чего-то не хватает — **не гадай** (см. §1.2).

---

## 1. Правила работы исполнителя

### 1.1 Clean room (юридически критично)

- Референс: <https://github.com/unxed/mcommander/tree/staging> (GPL, форк GNU mc 4.8.33 + Far-режим).
- Оттуда **можно** брать: поведение, UX, названия действий (action names), форматы файлов данных
  (skin `.ini`, `keymap.ini`, `ini`-настройки), раскладки клавиш, тексты-описания возможностей.
- Оттуда **нельзя** брать: код (даже «переведённый» построчно на Rust), структуру функций,
  комментарии, тексты сообщений/помощи дословно. Строки UI пишем свои.
- Порядок работы с референсом: прочитал → закрыл → описал поведение своими словами в комментарии
  к задаче/тесту → написал реализацию с нуля.
- Наши зависимости — только с лицензиями, совместимыми с BSD-3 (MIT, Apache-2.0, BSD, Zlib, Unicode).
  GPL/LGPL/MPL крейты не тянуть. Проверка: `cargo deny` (задача T-02).

### 1.2 Fail fast

- Не хватает информации (непонятное поведение референса, конфликт требований, не скачивается
  что-то, нет файла) → **остановиться и спросить пользователя**. Не делать «хоть как-то».
- Если вопрос не блокирующий — записать его в `docs/QUESTIONS.md` (формат в §10) и продолжать
  только ту часть, которая от него не зависит.
- Спорное решение, которое всё-таки пришлось принять, → в `docs/DECISIONS.md` + комментарий
  `// DECISION(D-xx): ...` в коде. Если у решения есть разумные альтернативы — вынести в настройку.

### 1.3 Объём и итеративность

- Никакого перфекционизма и попутного рефакторинга. Нашёл баг/кривизну вне задачи — строка в
  `docs/QUESTIONS.md` раздел «Замечено по пути», и обратно к задаче.
- Каждая задача должна оставлять программу собираемой и запускаемой (`cargo run` не падает).
- Каждая задача: тесты на новый код + обновление документации в репозитории.
- В ответе пользователю — только дельта (diff/новые файлы), без повтора ранее выданного.
- Сначала отдать патч. Сборку и тесты в своей песочнице гонять **только с разрешения пользователя**
  (CI — GitHub Actions, его не подменяем).
- Начиная с 10-го вызова инструмента в одном ответе — перед каждым вызовом спросить себя:
  не слишком ли глубоко зарылся, не держусь ли за любимую гипотезу, не оверинжиниринг ли это,
  есть ли способ проще. Если зашёл в тупик — дифференциальный диагноз: симптомы → все гипотезы →
  вычеркнуть несовместимые с симптомами.
- Логи (если нужны для отладки) — компактные, одна строка на событие, за флагом/env-переменной.
- Файл `filelist.md` **не трогать никогда** — его пишет другой процесс.
- Сообщение пользователя `)` = «одобряю, продолжай».

---

## 2. Цели и не-цели

### 2.1 Цель проекта
Двухпанельный текстовый файловый менеджер, функционально догоняющий M-Commander, на Rust,
с Far Manager-раскладкой клавиш по умолчанию. Классическая mc-раскладка — настройкой.
Периодически переносим новые возможности из апстрима (M-Commander) агентами; обратный перенос
идей приветствуется.

### 2.2 Целевые платформы
Linux (консоль, X/Wayland-терминалы, tmux, ssh), macOS, Windows (консоль/Windows Terminal),
Termux/Android. В перспективе — всё, что поддерживает оригинал (BSD и т.п.).
CI собирает и гоняет тесты на Linux, macOS, Windows; Android — только `cargo build --target aarch64-linux-android` (кросс-сборка, без тестов) начиная с итерации 1.

### 2.3 MVP (итерации 0–7)
Две панели, навигация, командная строка, файловые операции (F5/F6/F7/F8), Far-клавиши,
чтение скинов, keymap и ini-настроек в формате M-Commander.

### 2.4 Не-цели MVP (позже, см. §11)
Встроенный viewer (F3), редактор (F4), встроенный терминал/subshell, VFS (архивы, sftp, …),
панельные плагины, Lua API, diff viewer, mcstruct, mctree, help (F1), user menu (F2),
Info/QuickView/Tree панели, поиск файлов (Alt-F7), hotlist.
Для F3/F4 в MVP — запуск внешней программы (`$PAGER`/`$EDITOR`, на Windows — настройка).

### 2.5 Совместимость данных (обязательная)
- Скины `*.ini` M-Commander — читаем как есть.
- `keymap.ini` (секции `[filemanager]`, `[panel]`, `[dialog]`, `[input]`, `[menu]`, `[listbox]`, …) — читаем как есть; неизвестные действия/секции игнорируем с предупреждением в лог.
- `ini` (настройки) — читаем известные нам ключи, остальное сохраняем при перезаписи нетронутым.
- Lua API плагинов (`doc/LUA_API_REFERENCE.md` референса) — **после MVP**, но архитектура MVP
  обязана это допускать: все действия адресуются по строковому имени (§5.4), панели получают
  содержимое через трейт-провайдер (§5.5).

---

## 3. Ключевые решения

| ID | Решение | Почему | Где проверить |
|----|---------|--------|---------------|
| D-01 | Cargo workspace из нескольких крейтов (§4) | изоляция, быстрые тесты, явные границы | — |
| D-02 | crossterm — только для raw mode, размера терминала, вывода, и **ввода на Windows** | кроссплатформенность | — |
| D-03 | На Unix ввод читаем сами (сырые байты из stdin) и парсим своим декодером; **внутренний формат клавиатурных событий — Win32 InputEvent (как в far2l/f4/unxed/winkeys), см. docs/DECISIONS.md D-03** | crossterm не отдаёт неизвестные последовательности (APC far2l), а нам нужны far2l-расширения, kitty keyboard, win32-input-mode | **спайк T-03**: подтвердить; если crossterm умеет — упростить и записать в DECISIONS |
| D-04 | Собственный двойной cell-буфер + diff-рендер (не ratatui) | mc-подобные диалоги, тени, фокус, контроль над рамками и цветом | — |
| D-05 | Однопоточный UI-цикл + рабочие потоки для файловых операций, связь через каналы `std::sync::mpsc` | простота, без async-рантайма | — |
| D-06 | Все команды — строковые action names в стиле mc (`Copy`, `CdParent`, …) | keymap.ini работает как есть; будущий Lua вызывает те же действия | — |
| D-07 | Far-режим = оверлей поверх стандартного keymap; `far_mode = true` по умолчанию | так устроено в референсе, только дефолт другой | — |
| D-08 | Каталог настроек: свой `m5`, с чтением `mc6` как fallback (настраиваемо) | не портить конфиг M-Commander, но подхватывать его скины/keymap | §6.1, вопрос Q-02 |
| D-09 | Rust edition 2024, stable toolchain, MSRV фиксируется в T-01 | — | — |
| D-10 | Без `unsafe`, кроме платформенных обёрток в `m5-term`/`m5-fs` (помечать `// SAFETY:`) | — | — |

---

## 4. Структура репозитория

```
m5/
├── Cargo.toml                  # [workspace]
├── README.md
├── LICENSE                     # BSD-3
├── deny.toml                   # cargo-deny: лицензии
├── .github/workflows/ci.yml
├── docs/
│   ├── DESIGN.md               # этот файл
│   ├── PROGRESS.md             # статус задач, «где остановились»
│   ├── DECISIONS.md            # принятые спорные решения
│   ├── QUESTIONS.md            # открытые вопросы и «замечено по пути»
│   └── KEYS.md                 # таблица клавиш (генерируется руками из keymap, см. T-22)
├── crates/
│   ├── m5-term/      # терминал: raw mode, ввод (декодеры), вывод, Key, размеры, сигналы
│   ├── m5-ui/        # cell-буфер, рендер, виджеты, диалоги, скины (применение)
│   ├── m5-config/    # ini-парсер mc-формата, пути, keymap, skin-модель, настройки
│   ├── m5-fs/        # абстракция файловой системы: Provider-трейт, LocalFs, метаданные
│   ├── m5-ops/       # файловые операции: копирование, перенос, удаление, mkdir; прогресс; отмена
│   └── m5/           # бинарь: файловый менеджер (панели, командная строка, actions, main loop)
└── assets/
    ├── keymap.ini              # НАШ дефолтный keymap (свой файл, mc-формат)
    ├── keymap.far.ini          # НАШ Far-оверлей
    └── skins/default.ini       # НАШ дефолтный скин (свои цвета, mc-формат)
```

Зависимости между крейтами (строго ацикличны):
`m5-term ← m5-ui`, `m5-config ← m5-ui`, `m5-fs ← m5-ops`, всё ← `m5`.
`m5-term`, `m5-config`, `m5-fs` друг от друга не зависят.

Разрешённые внешние крейты (добавлять другие — только через DECISIONS):
`crossterm`, `unicode-width`, `unicode-segmentation`, `base64`, `thiserror`, `anyhow` (только в бинаре),
`libc` (unix), `signal-hook` (unix, SIGWINCH/SIGTSTP), `windows-sys` (если понадобится),
`tempfile` (dev), `filetime` (сохранение времён при копировании).

---

## 5. Архитектура

### 5.1 m5-term

> **Ревизия D-03:** внутренний формат ввода — `InputEvent` в стиле Win32 INPUT_RECORD
> (`crates/m5-term/src/key.rs`: VirtualKeyCode, ControlKeyState, Char, UnshiftedChar, KeyDown,
> RepeatCount, VirtualScanCode). Типы `Mods`/`KeyCode`/`Key`/`Event` ниже — исходный набросок,
> до T-04 не реализованный; как они соотносятся с `InputEvent` (например, `Key` только для keymap),
> решается в T-04 (см. Q-12 в docs/QUESTIONS.md).

```rust
pub struct Mods(u8); // SHIFT=1, ALT=2, CTRL=4, SUPER=8; битовые операции
pub enum KeyCode {
    Char(char),            // печатный символ; при Ctrl/Alt хранится базовая буква в нижнем регистре
    F(u8),                 // 1..=24
    Up, Down, Left, Right, Home, End, PageUp, PageDown, Insert, Delete,
    Enter, Tab, Backspace, Esc,
    Kp(KpKey),             // клавиши цифрового блока: Plus, Minus, Asterisk, Enter, Digit(u8)...
}
pub struct Key { pub code: KeyCode, pub mods: Mods }

pub enum Event { Key(Key), Paste(String), Mouse(MouseEvent), Resize(u16, u16), Wake }

pub trait Terminal {
    fn size(&self) -> (u16, u16);
    fn poll_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>>;
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>; // буфер, flush отдельно
    fn flush(&mut self) -> io::Result<()>;
    fn suspend(&mut self) -> io::Result<()>;  // отдать терминал дочернему процессу (§5.1.4)
    fn resume(&mut self) -> io::Result<()>;
    fn caps(&self) -> &Caps;                  // что удалось включить
}
```

Плюс `Waker` (клонируемый, `Send`), которым рабочие потоки будят `poll_event` → `Event::Wake`
(Unix — self-pipe; Windows — отдельный канал + короткий таймаут опроса, см. Q-03).

#### 5.1.1 Декодер ввода (Unix) — чистая функция, тестируется без терминала

```rust
pub struct Decoder { /* буфер незавершённой последовательности */ }
impl Decoder {
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<Event>);
    pub fn flush_timeout(&mut self, out: &mut Vec<Event>); // одиночный ESC по таймауту (~25 мс, настройка)
}
```
Поддерживаемые формы (по приоритету разбора):
1. far2l APC: `ESC _ f2l <base64> ESC \` или `... BEL`. Payload — стек, читается **с конца**,
   little endian; последний байт — команда. `K`/`k`: char u32, control-key-state u32, scan u16,
   vkey u16, repeat u16. `C`/`c`: char u16, cks u16, vkey u8. Заглавная — нажатие, строчная —
   отпускание (игнорируем). Ответы и пакеты resize — читать и выбрасывать. Маппинг Win32
   virtual key + control key state → `Key` — своя таблица (константы VK_* общеизвестны, Win32 API).
2. kitty keyboard protocol: `CSI <code>[:alt] ; <mods>[:event] u` и расширенные `CSI ... ~`/буквенные.
3. win32-input-mode: `CSI Vk;Sc;Uc;Kd;Cs;Rc _`.
4. Легаси xterm: `CSI`/`SS3` последовательности с модификаторами `;2..;8`, `ESC <char>` = Alt.
5. bracketed paste `CSI 200~ ... CSI 201~` → `Event::Paste`.
6. SGR mouse `CSI < b;x;y M/m`.
Плюс ini-секции `[terminal:<TERM>]` из `ini`/`defaults` (пользовательские переопределения
последовательностей) — **после MVP** (Q-04).

#### 5.1.2 Согласование возможностей при старте (Unix)
Отправить пачкой: запрос far2l `ESC _ far2l1 ESC \`, запрос kitty `CSI ? u`, затем Primary DA
`CSI c` как «стоп-маркер». Ждать до прихода ответа на DA (таймаут ~200 мс, настройка).
- Пришло `ESC _ far2lok ESC \` (или с BEL) → режим far2l, kitty/win32-input **не включать**.
- Иначе пришёл ответ kitty `CSI ? <flags> u` → включить `CSI > 1 u` (disambiguate), при выходе `CSI < u`.
- Иначе — легаси. (win32-input-mode `CSI ? 9001 h` — за настройкой, по умолчанию выкл, Q-05.)
- Не спрашивать far2l, если `TERM` начинается с `screen` или `tmux`, или задано `M5_FAR2L=0`.
- Все ответы, пришедшие позже, декодер должен молча проглатывать.

#### 5.1.3 Windows
Ввод — через crossterm `event::read()` (ReadConsoleInput даёт полные модификаторы),
конвертация в наш `Event`. Если в Windows Terminal нужен far2l — после MVP.

#### 5.1.4 Отдача терминала дочернему процессу (`suspend`/`resume`)
`suspend`: выключить kitty (`CSI < u`), far2l (`ESC _ far2l0 ESC \`), мышь, bracketed paste,
показать курсор, выйти из alternate screen, выключить raw mode.
`resume`: всё обратно + полная перерисовка. То же — при выходе и при панике (panic hook!).

#### 5.1.5 Вывод
Только через наш рендер (§5.2). Цвет: 16 / 256 / truecolor — определяется из `COLORTERM`,
`TERM` и настройки; скин с `256colors = true` на 16-цветном терминале → деградация к ближайшему.

### 5.2 m5-ui

#### 5.2.1 Cell-буфер и рендер
```rust
pub struct Style { pub fg: Color, pub bg: Color, pub attrs: Attrs } // Attrs: BOLD|UNDERLINE|ITALIC|REVERSE|BLINK
pub struct Cell { pub text: CompactString-или-String, pub width: u8, pub style: Style }
pub struct Buffer { w: u16, h: u16, cells: Vec<Cell> }
impl Buffer {
    pub fn put_str(&mut self, x: u16, y: u16, s: &str, style: Style, max_w: u16) -> u16; // учитывает ширину графем
    pub fn fill(&mut self, rect: Rect, ch: char, style: Style);
    pub fn frame(&mut self, rect: Rect, lines: &LineSet, style: Style, double: bool);
    pub fn shadow(&mut self, rect: Rect, style: Style); // тень диалога как в mc
}
pub fn diff_render(prev: &Buffer, next: &Buffer, out: &mut Vec<u8>, caps: &Caps);
```
- Графемы/ширины: `unicode-segmentation` + `unicode-width`. Широкий символ занимает 2 ячейки,
  вторая — «продолжение» (`width = 0`).
- Снапшот-тесты: `Buffer::to_text()` (символы) и `to_style_map()` (буквенные коды стилей) →
  сравнение с ожидаемыми строками в тесте.

#### 5.2.2 Модель виджетов (mc-подобная, без retained-фреймворка)
```rust
pub trait Widget {
    fn render(&self, buf: &mut Buffer, area: Rect, ctx: &RenderCtx);
    fn handle(&mut self, ev: &UiEvent, ctx: &mut UiCtx) -> Handled; // Handled::{Yes, No, Action(String)}
    fn focusable(&self) -> bool { false }
    fn cursor(&self) -> Option<(u16, u16)> { None }
}
```
- `Dialog` — стек модальных окон (`Vec<Box<dyn Dialog>>`), верхний получает ввод.
  Диалог = рамка + заголовок + набор виджетов с фокусом (Tab/Shift-Tab, стрелки по keymap `[dialog]`).
- Обязательные виджеты MVP: `Label`, `Input` (однострочный, история, keymap `[input]`), `Button`,
  `Checkbox`, `RadioGroup`, `Listbox`, `MenuBar` + `PopupMenu` (F9), `ButtonBar` (нижняя строка
  F1–F10, меняется от модификаторов), `Gauge` (прогресс), `MessageBox` (Ok / Yes-No / варианты).
- Far-особенности диалогов (при `far_mode`): Ctrl-Enter = действие по умолчанию, `+`/`-` включают/
  выключают чекбокс.
- Хоткеи кнопок/пунктов: символ после `&` в подписи, подсветка стилем `*hotkey`.

#### 5.2.3 Скин (применение)
`m5-config` даёт модель скина (§6.3); `m5-ui` строит из неё `Theme { get(section, key) -> Style }`
с fallback: `section.key` → `core.key` → `core._default_`.

### 5.3 m5-config

- **Парсер ini mc-формата** (свой, ~200 строк): секции `[name]`, пары `key = value` (пробелы
  вокруг `=` и ведущие отступы допустимы), комментарии `#` и `;` в начале строки, значения —
  сырые строки (UTF-8, могут содержать `;`), повторный ключ — последний побеждает,
  сохранение **порядка** секций и ключей и неизвестных ключей при записи. Экранирование —
  сверить с референсом (Q-06).
- Пути (§6.1), настройки (§6.2), скины (§6.3), keymap (§6.4).

### 5.4 Действия (actions) и keymap

```rust
pub struct Keymap { sections: HashMap<String /*section*/, HashMap<Key, String /*action*/>> }
impl Keymap {
    pub fn lookup(&self, section: &str, key: &Key) -> Option<&str>;
    pub fn keys_for(&self, section: &str, action: &str) -> Vec<Key>; // для ButtonBar/меню
}
```
Сборка итогового keymap:
1. встроенный `assets/keymap.ini` (наш);
2. если `far_mode` — поверх него `assets/keymap.far.ini` (для перечисленных действий заменяет **весь** список клавиш действия; Q-07);
3. пользовательский `keymap.ini` из каталога настроек (D-08) — заменяет списки клавиш для указанных действий.
Если одна клавиша оказалась у двух действий в одной секции — побеждает более поздний слой,
предупреждение в лог.

Синтаксис значения: `key1; key2; ...`. Имя клавиши: модификаторы `ctrl-`, `alt-`, `shift-`
(в любом порядке, регистр не важен) + базовое имя: буква/цифра, `f1`..`f24`, `up`, `down`, `left`,
`right`, `home`, `end`, `pgup`, `pgdn`, `insert`, `delete`, `enter`, `tab`, `backspace`, `esc`,
`space`, keypad `kpplus`, `kpminus`, `kpasterisk`, `a1`/`c1` …, и словесные имена знаков
(`backslash`, `question`, `comma`, `dot`, `equal`, `prime`, `lt`, `gt`, `exclamation`, `plus`, `minus`,
`asterisk`, `lbrace`, `rbrace`, `lparenthesis`, `rparenthesis` …).
**Полную таблицу словесных имён составить, прочитав список имён в `lib/tty/key.c` референса**
(это формат данных, не код). Конвенция mc: `f11`..`f20` в keymap означают Shift-F1..Shift-F10 —
**проверить по референсу** и зафиксировать в `docs/KEYS.md`; странности вида `f21`, `f22` в
Far-таблице → Q-08.

Действия реализуются в бинаре как `fn(&mut App, &ActionArgs) -> Result<()>` в реестре
`HashMap<&'static str, ActionFn>`. Неизвестное действие из keymap — предупреждение, не ошибка.

### 5.5 m5-fs

```rust
pub struct Entry {
    pub name: OsString, pub kind: Kind /*File|Dir|Symlink{target_kind}|Other*/,
    pub size: u64, pub mtime: SystemTime, pub atime: Option<SystemTime>, pub ctime: Option<SystemTime>,
    pub mode: Option<u32>, pub uid: Option<u32>, pub gid: Option<u32>, pub owner: Option<String>,
    pub hidden: bool, // unix: имя на '.'; windows: атрибут HIDDEN
}
pub trait Provider: Send + Sync {
    fn list(&self, dir: &Path) -> io::Result<Vec<Entry>>;
    fn stat(&self, p: &Path) -> io::Result<Entry>;
    fn roots(&self) -> Vec<PathBuf>; // "/" на unix; диски на windows (Alt-F1/F2 после MVP)
}
pub struct LocalFs;
```
Имена файлов — `OsString` везде; в UI выводятся через lossy-преобразование, но операции идут
по исходным байтам (имена с невалидным UTF-8 обязаны копироваться/удаляться — тест).
Провайдер — точка расширения для будущих VFS/панельных плагинов (архивы, sftp, Lua-панели).

### 5.6 m5-ops

```rust
pub enum OpKind { Copy, Move, Delete, MkDir }
pub struct OpRequest { kind: OpKind, sources: Vec<PathBuf>, target: Option<PathBuf>, opts: OpOptions }
pub struct OpOptions { follow_links: bool, preserve_attrs: bool, dive_into_subdir: bool, /* ... */ }

pub enum OpEvent {                                   // поток worker → UI
    Progress { file: PathBuf, file_done: u64, file_total: u64, bytes_done: u64, bytes_total: u64, files_done: u64, files_total: u64 },
    Ask(Question, Sender<Answer>),                   // конфликт / ошибка → UI спрашивает пользователя
    Done(Summary), Failed(String),
}
pub enum Question { Overwrite { src: Entry, dst: Entry }, Error { path: PathBuf, err: String } }
pub enum Answer { Yes, No, All, None, Skip, SkipAll, Retry, Abort, Append, OverwriteIfNewer /*...*/ }

pub fn spawn(req: OpRequest, events: Sender<OpEvent>, cancel: Arc<AtomicBool>, waker: Waker) -> JoinHandle<()>;
```
Правила (поведение по мотивам mc, реализация своя):
- Подсчёт объёма (`files_total`, `bytes_total`) — первым проходом, отменяемым.
- Копирование файла — буфером (по умолчанию 1 МиБ), проверка `cancel` между блоками.
- Move: сначала `rename`; при `EXDEV` (другое устройство) — copy+delete.
- Копирование каталога в себя/свой подкаталог — запретить с ошибкой до начала.
- Симлинки: по умолчанию копируются как симлинки (`follow_links=false`).
- Сохранение атрибутов: права, mtime/atime (`filetime`); владелец — пытаться, ошибку молча игнорировать.
- Удаление каталога — рекурсивно; перед этим диалог подтверждения (в UI), для непустого каталога — второй.
- Все операции тестируются на `tempfile::TempDir` без UI: тест подаёт заранее заданные `Answer`.

### 5.7 m5 (бинарь)

```
App {
  term: Box<dyn Terminal>, theme, keymap, settings,
  panels: [Panel; 2], active: Side,
  cmdline: Input, dialogs: DialogStack,
  ops: Option<RunningOp>,          // MVP: одна операция за раз, модальный прогресс
  screen: (Buffer, Buffer),
}
```
Главный цикл: `poll_event` → если открыт диалог, событие ему; иначе: keymap-секция `[panel]`, затем
`[filemanager]` (+ `[filemanager:xmap]` после префикса ExtendedKeyMap), иначе — в командную строку
(`[input]`); печатные символы без модификаторов — в командную строку. Затем render → diff → flush.

Раскладка экрана (MVP): строка меню (видна по F9 или всегда — настройка), две панели 50/50
(вертикальное разделение), командная строка с приглашением `путь>`, строка кнопок F1–F10.

**Panel**: путь, список `Entry` (+ `..`), курсор, верхняя видимая строка, множество отмеченных,
режим сортировки + обратный порядок, режим листинга (Full / Brief / Long на MVP; таблицы режимов
Ctrl-1..Ctrl-0 → Q-09), «показывать скрытые», мини-статус (имя/размер/дата файла под курсором),
итог отмеченных (кол-во, суммарный размер). Каталоги — всегда сверху; `..` — первым.
Сортировка имён — с учётом регистра или без (настройка, по умолчанию без), «натуральная» — Q-10.

**Командная строка**: Enter с непустой строкой — выполнить через `$SHELL -c` (Windows: `cmd /C`,
настройка), `cd <path>` обрабатывается сами (без шелла, с `~` и относительными путями), затем
`suspend` → запуск → ожидание → «Press any key» (настройка) → `resume` → перечитать панели.
История командной строки — в памяти + файл в каталоге данных.

**Ctrl-O (Shell)** в MVP: `suspend` и запуск интерактивного `$SHELL`; по выходу из шелла — `resume`.
Синхронизации каталога нет (это задача встроенного терминала, после MVP). → Q-11.

---

## 6. Данные и форматы

### 6.1 Пути
| Назначение | Unix (XDG) | Windows | Termux |
|---|---|---|---|
| настройки | `$XDG_CONFIG_HOME/m5` (по умолч. `~/.config/m5`) | `%APPDATA%\m5` | как Unix |
| данные (история) | `$XDG_DATA_HOME/m5` | `%LOCALAPPDATA%\m5` | как Unix |
| кэш | `$XDG_CACHE_HOME/m5` | `%LOCALAPPDATA%\m5\cache` | как Unix |
Fallback-чтение (D-08, настройка `compat_mc6_dirs`, по умолчанию `true`): если файла (`ini`,
`keymap.ini`, `skins/<name>.ini`) нет в каталоге m5 — искать в `~/.config/mc6` (на Unix).
Запись — **только** в каталог m5. Переменная `M5_CONFIG_DIR` переопределяет всё (для тестов).

### 6.2 Настройки (`ini`)
Файл с именем `ini` (как у референса). На MVP используем секцию, где лежат общие флаги, и ключи:
`far_mode` (bool, **по умолчанию true** — отличие от референса), `skin` (имя), а также ключи
показа скрытых файлов, подтверждений (удаление, перезапись, выход), «пауза после выполнения
команды», режимов листинга и сортировки панелей. **Точные имена секций и ключей взять из
`src/setup.c` референса** и выписать в `docs/SETTINGS.md` (задача T-12). Наши собственные новые
ключи (`compat_mc6_dirs`, `esc_timeout_ms`, `term_query_timeout_ms`, `win32_input_mode`, …) —
в отдельной секции `[m5]`, чтобы не мешать M-Commander, если файл общий.
bool в ini: `true`/`false` и `1`/`0` — читать оба, писать как у референса (сверить, Q-06).

### 6.3 Скины
Формат — ini (§5.3). Секции, которые нужны MVP: `[skin]` (`description`, `256colors`, `truecolors`?),
`[lines]` (символы рамок: `horiz`, `vert`, `lefttop`, …, двойные `d*`), `[core]`, `[dialog]`,
`[error]`, `[filehighlight]`, `[menu]`, `[popupmenu]`, `[buttonbar]`, `[statusbar]`, `[widget-panel]`,
`[widget-scrollbar]`. Остальные секции (`[editor]`, `[viewer]`, `[mcterm]`, `[mcstruct-*]` …)
парсятся и хранятся, но не используются.
Значение цвета: `fg;bg;attrs` — любая часть может быть пустой (= унаследовать из `core._default_`).
Имена цветов (`black`, `lightgray`, `gray`, `brightred`, `colorNNN`, `rgbRGB`, `#rrggbb`, `default`)
и атрибутов (`bold`, `underline`, `italic`, `reverse`, `blink` …) — **сверить со списком в
`lib/tty/color.c`/`color-internal.c` референса** (это формат данных). Пример (из скина референса):
```
[core]
    _default_ = lightgray;black
    selected = black;cyan
    marked = yellow;black
```
`[filehighlight]` ссылается на группы из `filehighlight.ini` (маски/типы файлов → цвет) — на MVP
поддержать группы по типу (каталог, исполняемый, симлинк, «битый» симлинк, устройство) и по
расширению-маске. Файл `filehighlight.ini` — свой в `assets/`, совместимого формата.
Встроенный дефолтный скин — **свой**, в духе Far (синие панели, бирюзовый курсор).

### 6.4 keymap
См. §5.4. В `assets/keymap.ini` действия и секции — те же имена, что в референсе; привязки
классические (mc). В `assets/keymap.far.ini` — Far-оверлей:

| Секция | Действие | Клавиши |
|---|---|---|
| filemanager | Find | alt-f7; alt-question |
| filemanager | History (командной строки) | alt-f8; alt-h |
| filemanager | EditorViewerHistory | alt-f11; alt-shift-e |
| filemanager | PanelInfo / PanelQuickView / PanelTree | ctrl-l / ctrl-q / ctrl-t |
| filemanager | PutCurrentFullSelected | ctrl-f; ctrl-shift-enter |
| filemanager | ChangeMode (атрибуты) | ctrl-a |
| filemanager | ExtendedKeyMap (префикс) | alt-x |
| filemanager | ApplyCommand | ctrl-g |
| filemanager | Link (жёсткая) / Tree | alt-f6 / alt-f10 |
| filemanager | SaveSetup | Shift-F9 (имя клавиши — Q-08) |
| filemanager | MenuLastSelected | Shift-F10 (Q-08) |
| filemanager | HotList | alt-backslash |
| dialog | Ok | enter; ctrl-enter |
| panel | Mark | insert |
| panel | History (каталогов) | alt-shift-h; alt-f12 |
| panel | SortByName / Ext / MTime / Size | ctrl-f3 / ctrl-f4 / ctrl-f5 / ctrl-f6 |
| panel | SortByUnsorted / CTime / ATime / Owner | ctrl-f7 / ctrl-f8 / ctrl-f9 / ctrl-f11 |
| panel | Sort (меню сортировки) | ctrl-f12 |
| panel | CdRoot | ctrl-backslash |
| panel | PanelListingMode1..10 | ctrl-1 .. ctrl-0 |
| panel | быстрый поиск | alt-<символ>; следующее/предыдущее совпадение ctrl-enter / ctrl-shift-enter |
| input | End | alt-gt; end; c1 |
| input | HistoryPrev / HistoryNext | alt-p; ctrl-down; ctrl-e / alt-n; ctrl-up; ctrl-x |
| input | Clear / Yank | ctrl-y / alt-y |
| input | DeleteToWordBegin / End | alt-backspace; ctrl-backspace / alt-d; ctrl-delete |

(Секции editor/viewer — после MVP.) Ctrl-цифра различима только с far2l/kitty/win32-input или на
Windows; на легаси-терминале — недоступна (это ожидаемо, пометить в KEYS.md).

---

## 7. Нефункциональные требования
- Каталог на 100 000 файлов: чтение + сортировка + первый кадр < 1 с (release), прокрутка без задержек.
- Отметка всех файлов (Insert-серия, `*`) на 100 000 — < 100 мс.
- UI не блокируется во время файловых операций (worker + прогресс-диалог с «Отмена»).
- Ни одна паника не оставляет терминал в raw/alternate-режиме (panic hook → `suspend`).
- Без утечек терминальных режимов дочерним процессам (§5.1.4).

---

## 8. Тестирование
- Юнит-тесты в каждом крейте, рядом с кодом. Обязательные области:
  - `m5-term::Decoder`: таблицы «байты → события» для каждой формы ввода (§5.1.1), включая
    разрезанные посередине последовательности (`feed` по одному байту) и мусор.
  - `m5-config`: парсер ini (отступы, комментарии, повторы, порядок при записи), имена клавиш
    (round-trip `Key` ↔ строка), сборка keymap со слоями, загрузка скина с fallback.
  - `m5-ui`: снапшоты буфера для каждого виджета; diff-рендер (две копии буфера → минимальный вывод).
  - `m5-fs`, `m5-ops`: на временных каталогах; невалидные UTF-8 имена (unix), симлинки, EXDEV
    (эмулировать нельзя — тестировать ветку через внедряемую ошибку), отмена, конфликты с ответами.
  - `m5`: тесты `App` без терминала — `FakeTerminal` (очередь событий на вход, буфер на выход):
    сценарии «нажали клавиши → состояние панели / снапшот экрана».
- Фикстуры скинов/keymap — **свои** (не копировать файлы референса в репозиторий).
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` на ubuntu/macos/windows,
  `cargo deny check licenses`, кросс-сборка aarch64-linux-android.

---

## 9. План итераций и задачи

Формат задачи: цель → что сделать → критерии приёмки (DoD). DoD **всегда** включает: тесты,
обновлённый `docs/PROGRESS.md`, зелёные `fmt`/`clippy`/`test` (проверяет CI пользователя).

### Итерация 0 — скелет и спайк ввода
- **T-01** Workspace, пустые крейты из §4, бинарь `m5` печатает версию. `rust-toolchain.toml`,
  MSRV в README. `docs/PROGRESS.md`, `DECISIONS.md` (перенести D-01..D-10), `QUESTIONS.md` (Q-xx из §12).
- **T-02** CI (`.github/workflows/ci.yml`) по §8, `deny.toml`.
- **T-03** Спайк D-03: бинарь `m5 --key-test` — raw mode, печатает сырые байты и декодированные
  события в одну строку на нажатие, выход по `q` трижды. Проверить, отдаёт ли crossterm APC/неизвестные
  последовательности. Результат записать в DECISIONS (D-03 подтверждено/пересмотрено).
  **Остановиться и показать результат пользователю** (это развилка архитектуры).

### Итерация 1 — терминал и рендер
- **T-04** (на базе `InputEvent`, см. ревизию D-03 в §5.1; `Key`/`Mods`/`Event` — по итогам Q-12) `Decoder` для легаси xterm + bracketed paste + SGR mouse. Тесты.
- **T-05** Декодер kitty + win32-input-mode. Тесты.
- **T-06** Декодер far2l APC + маппинг VK. Тесты с base64-фикстурами, собранными вручную по §5.1.1.
- **T-07** `UnixTerminal`: raw mode, alt screen, согласование возможностей (§5.1.2), SIGWINCH,
  `Waker`, suspend/resume, panic hook. `WindowsTerminal` через crossterm. `--key-test` переводится на них.
- **T-08** `Buffer`, `Style`, `diff_render`, 16/256/truecolor. Снапшот-тесты.

### Итерация 2 — первая видимая программа (read-only панели)
- **T-09** `m5-fs::LocalFs` + `Entry` (unix/windows). Тесты.
- **T-10** `Panel` (модель): чтение, сортировка по имени, каталоги сверху, `..`, курсор, прокрутка.
  Тесты модели без UI.
- **T-11** Экран: две панели (Full-режим: имя/размер/дата), мини-статус, строка кнопок; навигация
  стрелками/PgUp/PgDn/Home/End, Enter в каталог, `..`, Tab, F10 — выход. Захардкоженные клавиши и
  цвета (keymap и скины — следующая итерация). **Первый релиз для пользователей.**

### Итерация 3 — конфигурация
- **T-12** Парсер ini (§5.3), пути (§6.1), чтение `ini` → `Settings`; `docs/SETTINGS.md` с именами
  ключей из референса.
- **T-13** Имена клавиш ↔ `Key` (§5.4), `docs/KEYS.md`; `Keymap` со слоями; `assets/keymap.ini`,
  `assets/keymap.far.ini`; реестр действий; перевод T-11 на действия.
- **T-14** Модель скина + `Theme` + `assets/skins/default.ini`; применение к панелям/рамкам;
  параметр `--skin NAME` и ключ `skin` в ini; `filehighlight`.

### Итерация 4 — командная строка
- **T-15** Виджет `Input` (редактирование, keymap `[input]`, история, Far-клавиши ввода).
- **T-16** Командная строка: ввод печатных символов из панели, выполнение команды (§5.7), `cd`,
  PutCurrentPath/PutCurrentFullSelected (Ctrl-F, Ctrl-Enter, Alt-A …), история (Alt-F8 диалогом — после T-19).
- **T-17** Ctrl-O (Shell), Ctrl-R (Reread), Ctrl-U (Swap), Ctrl-\ (CdRoot), CdParent/CdChild.

### Итерация 5 — диалоги и отметка
- **T-18** Диалоговая подсистема: `DialogStack`, `Label`, `Button`, `Checkbox`, `RadioGroup`, `MessageBox`,
  фокус, тень, Far-поведение (Ctrl-Enter, +/−).
- **T-19** `Listbox` + диалог истории (командной строки Alt-F8, каталогов Alt-F12).
- **T-20** Отметка: Insert, Shift-стрелки, Select/Unselect/Invert (маска в диалоге), итог отмеченных.
- **T-21** Сортировки (Ctrl-F3..F12, обратный порядок повторным нажатием), режимы Brief/Long,
  показ скрытых, быстрый поиск (Alt-символ в Far-режиме; Ctrl-S/Alt-S в классическом).
- **T-22** `ButtonBar` с подписями из keymap и модификаторами (Alt/Ctrl/Shift меняют подписи).

### Итерация 6 — файловые операции
- **T-23** `m5-ops`: MkDir, Delete (рекурсивно, отмена, ошибки через `Ask`). Тесты на tempdir.
- **T-24** UI: F7 (диалог имени, создание вложенных путей), F8 (подтверждения), прогресс-диалог
  с `Gauge` и «Отмена».
- **T-25** `m5-ops`: Copy (файлы, каталоги, симлинки, атрибуты, конфликты, невалидные имена). Тесты.
- **T-26** `m5-ops`: Move (rename, EXDEV-фолбэк). Тесты.
- **T-27** UI: F5/F6 (диалог с целевым путём = путь другой панели, опции), конфликт-диалог
  (Перезаписать / Пропустить / Все / Новее / Дописать / Отмена), Shift-F5/Shift-F6 (одиночный файл,
  переименование на месте), обновление обеих панелей после операции, Alt-F6 (жёсткая ссылка).

### Итерация 7 — полировка MVP
- **T-28** F9: `MenuBar` (Левая / Файл / Команды / Настройки / Правая) — только пункты для
  реализованных действий; Shift-F10 последний пункт.
- **T-29** Диалог «Конфигурация» с чекбоксом Far-режима и основными флагами; Shift-F9 SaveSetup
  (запись ini с сохранением неизвестных ключей). Смена Far-режима применяется без перезапуска.
- **T-30** F3/F4 — внешний просмотрщик/редактор (настройка, `$PAGER`/`$EDITOR`, Windows — `notepad`/настройка).
- **T-31** Проверка нефункциональных требований (§7) на синтетическом каталоге, профилирование,
  исправление только того, что не проходит. README: установка, клавиши, совместимость.
  **Релиз MVP.**

---

## 10. Служебные файлы

`docs/PROGRESS.md`:
```
## Сейчас
Последняя закрытая: T-xx (дата) — одна строка что сделано.
Следующая: T-yy.
## История
- T-xx ✔ кратко; ссылки на ключевые файлы
```
`docs/QUESTIONS.md`:
```
## Открытые
- Q-xx [блокирует T-yy | не блокирует] Вопрос. Контекст. Варианты. Что сделано временно (+ настройка?).
## Замечено по пути
- файл:строка — что не так (не чиним в рамках текущей задачи)
## Закрытые
- Q-xx → ответ пользователя / ссылка на D-xx
```
`docs/DECISIONS.md`: `D-xx — решение — почему — альтернативы — где в коде`.

---

## 11. После MVP (порядок предварительный, детализировать отдельными документами)
1. Встроенный viewer (текст/hex, поиск, кодировки) — F3; затем Info/QuickView панели (Ctrl-L/Ctrl-Q).
2. Встроенный редактор (mcedit-подобный, Far-клавиши редактора из референса: Ctrl-F7 замена,
   Alt-F8 переход к строке, Ctrl-F3 номера строк, Ctrl-Z undo, Ctrl-A выделить всё, F4/Esc выход).
3. Встроенный терминал вместо subshell (синхронизация каталога панелей).
4. Поиск файлов (Alt-F7), Hotlist, Tree, user menu (F2), help (F1, Markdown).
5. VFS-слой поверх `m5-fs::Provider`: архивы, затем sftp/ftp.
6. Lua-рантайм (крейт `mlua`, проверить лицензию) с API, совместимым с `doc/LUA_API_REFERENCE.md`
   референса (воркспейсы `mc`, `mcedit`, `mcview`, `any`); панельные провайдеры из Lua.
7. Пользовательские последовательности `[terminal:<TERM>]`, learn keys.
8. Процесс «подтягивания» новых фич из апстрима агентами: регулярный разбор CHANGELOG.md
   референса → задачи в отдельный backlog (поведение, не код).

---

## 12. Открытые вопросы (стартовое содержимое `docs/QUESTIONS.md`)
- **Q-01** [не блокирует] Имя бинаря: `m5`? (принято временно).
- **Q-02** [не блокирует] D-08: свой каталог `m5` + чтение `mc6` как fallback — устраивает ли? Сделано настройкой.
- **Q-03** [T-07] Механизм `Waker` на Windows: таймаут опроса vs. отдельный поток чтения консоли.
- **Q-04** [не блокирует] Секции `[terminal:*]` пользовательских последовательностей — нужны ли в MVP?
- **Q-05** [не блокирует] win32-input-mode по умолчанию выключен — так?
- **Q-06** [T-12] Точные правила экранирования/кавычек и формат bool при записи ini у референса.
- **Q-07** [T-13] Far-оверлей: заменяет весь список клавиш действия или добавляет к нему? (сверить с референсом).
- **Q-08** [T-13] Имена клавиш `f19`, `f21`, `f22` в Far-таблице референса (Shift-F9 / Shift-F10) — какая конвенция?
- **Q-09** [T-21] Состав колонок режимов листинга Ctrl-1..Ctrl-0 (Far) — взять из референса или задать свои?
- **Q-10** [T-10] Натуральная сортировка (`file2` < `file10`) по умолчанию?
- **Q-11** [T-17] Ctrl-O в MVP — запуск `$SHELL` достаточен?
