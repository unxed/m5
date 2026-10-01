# T-03: что реально присылают терминалы в `m5 --key-test`

Только то, что видно в логах и скриншотах прогонов CI. Гипотезы помечены словом «предположение».

## Как проверялось

Workflow `.github/workflows/key-test.yml`: на раннере `ubuntu-24.04` поднимается Xvfb и оконный менеджер openbox,
в нём запускается настоящий терминал, внутри него `m5 --key-test` (релизная сборка). Клавиши шлёт `xdotool`
(XTEST), 76 нажатий: буквы, Shift+буква, кириллица (`xdotool type`), Ctrl/Alt/Ctrl+Alt/Shift-комбинации, F1-F12 и их
модификации, стрелки, Home/End/PgUp/PgDn/Insert/Delete (с модификаторами), Enter, Esc, Tab, Backspace, вставка
(Shift+Insert, в обоих буферах X лежит текст `pasted text from clipboard`) и `q` x3. Каждое нажатие сопоставляется
с «Raw:»-строками m5 по времени (`paired.txt`, `summary.txt` в артефакте; скриншоты `NN-*.png` после каждой группы).

Прогоны (артефакты `key-test-<терминал>-<режим>`, хранятся по умолчанию 90 дней):
- https://github.com/unxed/m5/actions/runs/36833342920 (режимы legacy, modifyOtherKeys, kitty-31, win32-9001);
- https://github.com/unxed/m5/actions/runs/36833927593 (то же + bracketed paste).

Терминалы: xterm и kitty из apt Ubuntu 24.04 (версии в логе прогона), WezTerm nightly (`wezterm-nightly.Ubuntu22.04.deb`).
Режим включается последовательностью, которую шаг шлёт в терминал через 3 с после старта m5 (после входа m5 в
альтернативный экран): `legacy` — ничего; `modifyother` — `ESC[>4;2m` (только xterm); `kitty-31` — `ESC[>31u`;
`win32-9001` — `ESC[?9001h`; `bracketed` — `ESC[?2004h`. Сам `m5 --key-test` ни один режим не запрашивает.

Вывод m5 для лога идёт через `tee`: с `script(1)` ввод до m5 не доходил (все 76 нажатий пропали), без него — доходит.

## Что делает сам m5 --key-test

Прогоны выше сделаны со старой версией `--key-test`; у неё были дефекты (скриншоты `04-fkeys.png`, `10-exit.png`):
вывод шёл «лесенкой» (строки кончались одним LF в raw-режиме), для всего, кроме одиночного печатного ASCII-байта,
печатались только «Raw:» и «Sequence: N bytes», а выход по `q q q` сравнивал блок с байтом `q` и в режиме kitty-31
(`q` = `ESC[113;;113u`) не срабатывал. Они исправлены (PR «возврат каретки… и выход по трём q», PR «показ
разобранных InputEvent»): строки кончаются CR LF, после «Raw:» печатаются строки «Event:» с событиями `Decoder`,
выход считает нажатия `q` по разобранным событиям. В прогоне 36838538559 (15 заданий) в каждом, включая kitty-31,
kitty-1 и WezTerm с kitty, строка `exit.txt` — «m5 exit after q q q: yes (terminal closed)».

## Что присылают терминалы без включения режимов (legacy)

Совпадает во всех трёх терминалах, если не указано иное:
- Буквы, цифры, знаки: сам символ. Shift+a: `41`, Shift+1: `21`. `я`: `d1 8f`.
- Ctrl+a: `01`, Ctrl+z: `1a`, Ctrl+[: `1b` (то же, что Esc), Ctrl+/: `1f`, Ctrl+Space: `00`.
- Enter, Shift+Enter, Ctrl+Enter: все три — `0d` (различить нельзя). Tab: `09`, Shift+Tab: `ESC[Z`, Esc: `1b`.
- BackSpace: `7f`, Ctrl+BackSpace: `08`.
- F1-F4: `ESC O P/Q/R/S`; F5..F12: `ESC[15~ 17~ 18~ 19~ 20~ 21~ 23~ 24~`.
- Стрелки `ESC[A/B/D/C`, Home `ESC[H`, End `ESC[F`, PgUp `ESC[5~`, PgDn `ESC[6~`, Insert `ESC[2~`, Delete `ESC[3~`.
- С модификаторами (Shift=2, Alt=3, Ctrl=5, Ctrl+Shift=6): `ESC[1;<m>A/D/H`, `ESC[3;<m>~`, `ESC[15;<m>~` для F5.
- Alt+буква/цифра/BackSpace:
  - xterm (настройки по умолчанию): байт символа с выставленным старшим битом, в UTF-8: Alt+a = `c3 a1`, Alt+z = `c3 ba`,
    Alt+1 = `c2 b1`, Alt+BackSpace = `c3 bf`, Alt+Shift+a = `c3 81`, Ctrl+Alt+a = `c2 81`;
  - kitty и WezTerm: `ESC` + символ: Alt+a = `1b 61`, Alt+BackSpace = `1b 7f`, Alt+Shift+a = `1b 41`, Ctrl+Alt+a = `1b 01`.
- Вставка (Shift+Insert) без bracketed paste: просто байты текста (26 байт одним блоком), без обрамления.

Потеряно или не отличимо (legacy):
- Alt+Enter: в xterm и WezTerm до m5 ничего не дошло; в kitty пришло `1b 0d`. Причина не выяснена (предположение:
  сочетание занято самим терминалом, например переключением полноэкранного режима).
- Ctrl+Shift+1 и Ctrl+Shift+F5: в kitty ничего не дошло (предположение: стандартные сочетания kitty); в xterm
  Ctrl+Shift+1 пришло как `21` (Ctrl потерян), Ctrl+Shift+F5 как `ESC[15;6~`; в WezTerm Ctrl+Shift+1 не дошло, Ctrl+Shift+F5 как `ESC[15;6~`.
- Ctrl+Shift+y: xterm и WezTerm `19` (то же, что Ctrl+y), kitty `ESC[121;6u` (это CSI u даже без запроса режима).
- Ctrl+Enter и Shift+Enter не отличаются от Enter (во всех трёх).

## modifyOtherKeys (xterm, `ESC[>4;2m`)

Клавиши с модификаторами приходят как `ESC[27;<mod>;<код>~`: Ctrl+a `ESC[27;5;97~`, Ctrl+Enter `ESC[27;5;13~`,
Shift+Enter `ESC[27;2;13~`, Ctrl+Space `ESC[27;5;32~`, Ctrl+[ `ESC[27;5;91~`, Alt+a `ESC[27;3;97~`, Alt+BackSpace
`ESC[27;3;127~`, Ctrl+Alt+a `ESC[27;7;97~`, Shift+a `ESC[27;2;65~`, Ctrl+Shift+1 `ESC[27;6;33~`. Alt+Enter по-прежнему не дошло.
Остальные клавиши (F-клавиши, стрелки, Enter без модификаторов) — как в legacy.

## kitty-протокол (`ESC[>31u`: флаги 1+2+4+8+16)

- **kitty**: последовательности kitty-протокола доходят до приложения: проверено побайтно в `summary.txt`
  прогона kitty / kitty-31. Примеры: `a` -> `ESC[97;;97u` и отпускание `ESC[97;1:3u`; Enter -> `ESC[13u` +
  `ESC[13;1:3u`; Esc -> `ESC[27u`; Tab -> `ESC[9u`; BackSpace -> `ESC[127u`; Shift+a -> `ESC[97:65;2;65u`;
  Ctrl+a -> `ESC[97;5u`; Alt+a -> `ESC[97;3u`; `я` -> `ESC[1103;;1103u`; F1 -> `ESC[P`, F2 `ESC[Q`, F3 `ESC[13~`, F4 `ESC[S`,
  F5 `ESC[15~`; стрелка вверх `ESC[A` (+ отпускание `ESC[1;1:3A`); Shift+Up `ESC[1;2A`.
  Отдельными событиями приходят нажатие и отпускание модификаторов: `ESC[57441;2u` (левый Shift), `ESC[57442;5u` (левый Ctrl),
  `ESC[57443;3u` (левый Alt), и их отпускания `...;1:3u`.
  Каждое нажатие даёт как минимум два блока: нажатие и отпускание (`...;1:3`).
- Вставка в этом режиме без bracketed paste: байты текста.
- **WezTerm**: при `ESC[>31u` вывод тот же, что в legacy (ни одной `CSI u`-последовательности). Предположение: в
  WezTerm kitty-протокол по умолчанию выключен настройкой. В прогоне это не проверялось.
- **xterm**: kitty-протокол не поддерживает, не запускалось.

## win32-input-mode (`ESC[?9001h`)

- kitty и WezTerm: вывод m5 идентичен legacy (ни одной последовательности вида `ESC[Vk;Sc;Uc;Kd;Cs;Rc_`). То есть при такой
  настройке (этот запрос, без конфигурации терминала) терминал win32-input-mode не включил. Предположение для WezTerm: режим
  отключён настройкой по умолчанию. Не проверялось.
- Терминала с рабочим win32-input-mode в этих прогонах не было, поэтому прохождение этих последовательностей до приложения
  **не проверено**. Не проверялся и xterm.

## far2l (APC)

**Не проверено.** В прогонах не было терминала, который шлёт far2l-расширение (`ESC _ f4l ... ESC \`): ни один из xterm, kitty, WezTerm
его не реализует, а терминал far2l (GUI-версия на wx или TTY-режим) в CI не ставился. Поэтому ответ на вопрос «доходит ли APC до приложения»
этими прогонами не получен. Не проверялось и то, как crossterm обходится с APC.

## Bracketed paste (`ESC[?2004h`)

xterm, kitty и WezTerm одинаково: Shift+Insert приходит одним блоком `ESC[200~pasted text from clipboard ESC[201~`
(прогон 36833927593, строка Shift+Insert в `summary.txt`).

## Сверка разобранных InputEvent с ожидаемыми (прогон 36838538559)

Прогон: https://github.com/unxed/m5/actions/runs/36838538559 (15 заданий: xterm legacy/modifyOtherKeys/bracketed;
kitty legacy/kitty-1/kitty-31/win32-9001/bracketed; WezTerm legacy/modifyOtherKeys/kitty-31/kitty-31-cfg/win32-9001/
win32-9001-cfg/bracketed). `.github/key-test/compare.py` сопоставляет по времени каждую клавишу с первым событием
нажатия (кроме самих модификаторов) в выводе m5 и сверяет VK, символ и Shift/Alt/Ctrl с ожидаемыми (как дала бы консоль
Windows; `?` в таблице — не проверялось). Итог по заданиям (строка `compare:` в `compare.txt` артефакта; всего 73 проверки, Ctrl+Return в списке дважды):

| терминал / режим | совпало | расхождений | не дошло |
|---|---|---|---|
| xterm legacy, bracketed | 58 | 14 | 1 |
| xterm modifyOtherKeys | 70 | 2 | 1 |
| kitty legacy, bracketed, win32-9001 | 65 | 6 | 2 |
| kitty kitty-1 (`ESC[>1u`) | 70 | 1 | 2 |
| kitty kitty-31 | 70 | 3 | 0 |
| WezTerm legacy, bracketed, kitty-31 (без настройки), win32-9001, win32-9001-cfg | 64 | 7 | 2 |
| WezTerm modifyOtherKeys | 70 | 1 | 2 |
| WezTerm kitty-31-cfg | 70 | 3 | 0 |

Что совпало: буквы, цифры, знаки, `я`, Shift+буква, Ctrl+буква, F1-F12 (в том числе с Shift/Ctrl/Alt), стрелки,
Home/End/PgUp/PgDn/Insert/Delete с Shift/Ctrl/Alt, Enter, Esc, Tab, Shift+Tab, BackSpace — декодер дал ожидаемые VK,
символ и модификаторы в тех режимах, где терминал присылает достаточно информации (см. ниже).
Вставка (Shift+Insert): во всех 15 заданиях текст `pasted text from clipboard` пришёл как последовательность событий
клавиш; в bracketed-заданиях (xterm, kitty, WezTerm) обрамлён ровно одной парой `Paste{START}` / `Paste{END}`, в остальных
режимах маркеров нет.

Расхождения, причина которых в терминале (в событии нет данных, из которых их можно восстановить):
- legacy (xterm, kitty, WezTerm): Ctrl+Enter, Shift+Enter, Ctrl+BackSpace (`08`), Ctrl+[ (`1b`) приходят теми же байтами,
  что Enter, BackSpace, Esc; Ctrl+Shift+y приходит как `19`, то есть Shift теряется; Ctrl+Shift+1 в xterm приходит как `21`
  (Shift без Ctrl). `1f` (Ctrl+/) декодер показывает как VK_OEM_MINUS с Ctrl (тот же байт даёт Ctrl+-, Ctrl+_).
- xterm без настройки: Alt+буква приходит как буква с выставленным старшим битом в UTF-8 (`c3 a1` для Alt+a), декодер
  видит символ `á` без Alt и без VK (ту же последовательность даёт набор `á`). Alt+Return в xterm и WezTerm до m5 не дошло
  ни в одном режиме (в kitty legacy пришло `1b 0d`).
- kitty и WezTerm: Ctrl+Shift+1 и Ctrl+Shift+F5 (kitty) и Ctrl+Shift+1 (WezTerm) не дошли или пришли только события
  модификаторов; в kitty-31 в `compare.txt` у них «no key press event», в сырых байтах — только Ctrl/Shift
  (`ESC[57442;5u`, `ESC[57441;6u`) и их отпускание. Предположение: сочетания заняты самим терминалом.
- WezTerm kitty-31-cfg: Alt+Return — пришли нажатие/отпускание Alt и отпускание Enter (`ESC[13;1:3u`), нажатия Enter нет.
  Предположение: Alt+Enter занято терминалом.
- modifyOtherKeys (xterm и WezTerm) и kitty-протокол: Ctrl+/ приходит как `ESC[27;5;47~` / `ESC[47;5u`; декодер даёт символ
  `/` с Ctrl (для Ctrl+буква он даёт управляющий символ). Это решение декодера, не потеря; для keymap достаточно VK и Ctrl.

Расхождение декодера, найденное этим прогоном и исправленное (PR «текст при зажатом Ctrl не подменяет управляющий символ»):
WezTerm с `enable_kitty_keyboard=true` и `ESC[>31u` присылает Ctrl+a как `ESC[97;5;97u` (с текстом `a`; kitty шлёт `ESC[97;5u`),
а декодер ставил символ `a` вместо `\x01` (Ctrl+a, Ctrl+z, Ctrl+[, Ctrl+Shift+y, Ctrl+Alt+a). В прогоне 36837731856 это давало
8 расхождений, в 36838538559 (после исправления) осталось 3 — все три перечислены выше. Регрессионные тесты —
`text_of_ctrl_combinations_is_ignored` в `crates/m5-term/src/decode/tests/kitty_cases.rs` на захваченных байтах.

Другие наблюдения:
- WezTerm (nightly, прогон 36838538559) без настройки на `ESC[>31u` не отвечает kitty-последовательностями; с
  `--config enable_kitty_keyboard=true` отвечает (`ESC[97;5;97u` и т.д.). Нажатие Ctrl он отправляет как `ESC[57442;1u`
  (поле модификаторов 1), kitty — `ESC[57442;5u`; декодер обрабатывает оба.
- WezTerm принимает modifyOtherKeys (`ESC[>4;2m`): Ctrl+a приходит как `ESC[27;5;97~`, результат такой же, как в xterm
  (70 совпадений из 73, расхождения — Alt+Return и Ctrl+Shift+1 не дошли, Ctrl+/ как описано выше).
- win32-input-mode: ни в kitty, ни в WezTerm (в том числе с `--config allow_win32_input_mode=true`) после `ESC[?9001h`
  ни одной последовательности `ESC[Vk;Sc;Uc;Kd;Cs;Rc_` не пришло (вывод идентичен legacy, 64 / 65 совпадений). Терминала, который
  в этих прогонах реально шлёт win32-input-mode, нет; декодер win32 проверен только юнит-тестами.
- far2l APC: по-прежнему не проверено (нет терминала, который его шлёт).

## Чего прогоны не покрывают

Мышь, фокус, изменение размера, повторы клавиш (xdotool шлёт одиночные нажатия), раскладки, кроме одной кириллической буквы,
Windows/macOS-терминалы, vmlab: гости vmlab — Redox и Haiku, сборки m5 под них нет, поэтому использована та же техника
(Xvfb + xdotool + снимки экрана) прямо на раннере Linux.
