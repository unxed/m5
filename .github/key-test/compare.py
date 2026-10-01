#!/usr/bin/env python3
"""Compares the InputEvent that `m5 --key-test` printed for every key sent by
key-test.yml with the expected one (virtual key, character, Shift/Alt/Ctrl).

Usage: compare.py OUT_DIR   (OUT_DIR has keys.txt and typescript.raw.ts)

Prints one line per key: OK, DIFF (with expected, got and raw bytes) or NONE
(nothing arrived). It only reports; the exit status is always 0.
"""
import re
import sys

S, A, C = "Shift", "Alt", "Ctrl"
NAV = {"Up": 0x26, "Down": 0x28, "Left": 0x25, "Right": 0x27, "Home": 0x24,
       "End": 0x23, "Prior": 0x21, "Next": 0x22, "Insert": 0x2D, "Delete": 0x2E}

# label -> (virtual key or None, character or None, set of Shift/Alt/Ctrl)
# None means "not checked". Values are what a Win32 console would report.
EXP = {
    "a": (0x41, "a", set()), "b": (0x42, "b", set()), "z": (0x5A, "z", set()),
    "1": (0x31, "1", set()), "0": (0x30, "0", set()), "space": (0x20, " ", set()),
    "Shift+a": (0x41, "A", {S}), "Shift+1": (0x31, "!", {S}),
    "comma": (0xBC, ",", set()), "slash": (0xBF, "/", set()),
    "type-cyrillic-ya": (None, "я", set()),
    "Ctrl+a": (0x41, "\x01", {C}), "Ctrl+z": (0x5A, "\x1a", {C}),
    "Ctrl+bracketleft": (0xDB, "\x1b", {C}), "Ctrl+slash": (0xBF, "\x1f", {C}),
    "Ctrl+space": (0x20, None, {C}), "Ctrl+Return": (0x0D, None, {C}),
    "Ctrl+Shift+y": (0x59, "\x19", {C, S}), "Ctrl+Shift+1": (0x31, None, {C, S}),
    "Alt+a": (0x41, "a", {A}), "Alt+z": (0x5A, "z", {A}), "Alt+1": (0x31, "1", {A}),
    "Alt+BackSpace": (0x08, None, {A}), "Alt+Return": (0x0D, None, {A}),
    "Alt+Shift+a": (0x41, "A", {A, S}), "Ctrl+Alt+a": (0x41, "\x01", {C, A}),
    "Shift+F5": (0x74, None, {S}), "Ctrl+F5": (0x74, None, {C}),
    "Alt+F5": (0x74, None, {A}), "Ctrl+Shift+F5": (0x74, None, {C, S}),
    "Return": (0x0D, "\r", set()), "Shift+Return": (0x0D, None, {S}),
    "Escape": (0x1B, "\x1b", set()), "Tab": (0x09, "\t", set()),
    "Shift+Tab": (0x09, None, {S}), "BackSpace": (0x08, None, set()),
    "Ctrl+BackSpace": (0x08, None, {C}),
}
for n in range(1, 13):
    EXP[f"F{n}"] = (0x70 + n - 1, None, set())
for name, vk in NAV.items():
    EXP[name] = (vk, None, set())
for name in ("Up", "Left", "Home", "Delete"):
    EXP[f"Shift+{name}"] = (NAV[name], None, {S})
    EXP[f"Ctrl+{name}"] = (NAV[name], None, {C})
    EXP[f"Alt+{name}"] = (NAV[name], None, {A})

PASTE_LABEL = "Shift+Insert (paste)"
PASTE_TEXT = "pasted text from clipboard"
MODIFIER_VKS = {0x10, 0x11, 0x12, 0x14, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5}

ANSI = re.compile(r"\x1b\[[0-9;?<>=]*[A-Za-z]|\x1b[()][0-9A-Za-z]")
KEY = re.compile(
    r"Event: Key\{VK:0x([0-9A-Fa-f]+) Scan:0x[0-9A-Fa-f]+ Char:'(.*?)' (DOWN|UP) Mods:(\S+) Src:.*\}")


def parse_char(text):
    m = re.fullmatch(r"\\x([0-9A-F]{2})", text)
    return chr(int(m.group(1), 16)) if m else text


def show(vk, ch, mods):
    c = "?" if ch is None else ("\\x%02X" % ord(ch) if ord(ch) < 32 else ch)
    v = "?" if vk is None else "0x%04X" % vk
    return "VK:%s Char:'%s' Mods:%s" % (v, c, ",".join(sorted(mods)) or "-")


def main(out):
    keys = []
    for ln in open(out + "/keys.txt", encoding="utf-8", errors="replace"):
        ts, label, _spec = ln.rstrip("\n").split("|", 2)
        keys.append((float(ts), label))
    lines = []
    for ln in open(out + "/typescript.raw.ts", encoding="utf-8", errors="replace"):
        ln = ANSI.sub("", ln).replace("\r", "").rstrip("\n")
        ts, _, text = ln.partition(" ")
        try:
            lines.append((float(ts), text))
        except ValueError:
            pass
    ok = diff = none = 0
    for i, (ts, label) in enumerate(keys):
        end = keys[i + 1][0] if i + 1 < len(keys) else float("inf")
        mine = [t for (lt, t) in lines if ts <= lt < end]
        raws = [t[len("Raw: "):].split(" | ")[0] for t in mine if t.startswith("Raw: ")]
        raw = " ; ".join(raws)
        if label == PASTE_LABEL:
            ok, diff, none = paste(mine, raw, ok, diff, none)
            continue
        if label not in EXP:
            continue
        if not raws:
            none += 1
            print("NONE  %-22s nothing arrived" % label)
            continue
        got = None
        for t in mine:
            m = KEY.match(t)
            if m and m.group(3) == "DOWN" and int(m.group(1), 16) not in MODIFIER_VKS:
                mods = set()
                for part in m.group(4).split(","):
                    part = part.replace("Right", "")
                    if part in (S, A, C):
                        mods.add(part)
                got = (int(m.group(1), 16), parse_char(m.group(2)), mods)
                break
        evx, ecx, emx = EXP[label]
        if got is None:
            diff += 1
            print("DIFF  %-22s no key press event; raw: %s" % (label, raw))
            continue
        good = (evx is None or evx == got[0]) and (ecx is None or ecx == got[1]) and emx == got[2]
        if good:
            ok += 1
            print("OK    %-22s %s" % (label, show(*got)))
        else:
            diff += 1
            print("DIFF  %-22s expected %s | got %s | raw: %s"
                  % (label, show(evx, ecx, emx), show(*got), raw))
    print("compare: %d match, %d differ, %d nothing arrived" % (ok, diff, none))


def paste(mine, raw, ok, diff, none):
    if not mine:
        print("NONE  %-22s nothing arrived" % PASTE_LABEL)
        return ok, diff, none + 1
    starts = sum(1 for t in mine if t == "Event: Paste{START}")
    ends = sum(1 for t in mine if t == "Event: Paste{END}")
    text = ""
    for t in mine:
        m = KEY.match(t)
        if m and m.group(3) == "DOWN":
            text += parse_char(m.group(2))
    # Shift+Insert itself is not a key of the text; chars of the text only.
    good = text.endswith(PASTE_TEXT) and (starts, ends) in ((0, 0), (1, 1))
    brief = "Paste START/END: %d/%d, text events: %r" % (starts, ends, text)
    if good:
        print("OK    %-22s %s" % (PASTE_LABEL, brief))
        return ok + 1, diff, none
    print("DIFF  %-22s %s | raw: %s" % (PASTE_LABEL, brief, raw[:200]))
    return ok, diff + 1, none


if __name__ == "__main__":
    main(sys.argv[1])
