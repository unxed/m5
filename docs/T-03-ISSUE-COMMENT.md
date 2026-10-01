# T-03 Validation Complete (Unit Tests Passing)

## Status: ✅ UNIT TESTS PASSING | ⏳ AWAITING INTERACTIVE VALIDATION

### Architecture Decision: D-03 (Win32 InputEvent Format)

Successfully adopted Win32-compatible `InputEvent` format (matching unxed/winkeys used by far2l and f4).

---

## Implementation ✅

### Core Types (m5-term/src/key.rs)
- **InputEvent**: Universal event container (Key, Mouse, Focus, Paste, Far2l, Resize)
- **ControlKeyState**: Win32 modifier flags (LEFT_CTRL, RIGHT_CTRL, LEFT_ALT, RIGHT_ALT, SHIFT, ENHANCED, CAPS_LOCK, NUM_LOCK, SCROLL_LOCK)
- **EventType**: Enum for all event types
- **MouseButtonState**, **MouseEventFlags**: Mouse event constants
- Full Display impl for debugging/logging

### Interactive Tool (m5-term/src/key_test.rs)
- `m5 --key-test` displays InputEvent format
- Unix: raw stdin + InputEvent output
- Windows: crossterm event → InputEvent conversion
- Shows hex bytes + character representation

---

## Unit Test Coverage ✅

**35+ comprehensive tests, all passing:**

### ControlKeyState (8 tests)
- Individual modifier flags: LEFT_CTRL, RIGHT_CTRL, LEFT_ALT, RIGHT_ALT, SHIFT, ENHANCED
- Modifier combinations (Ctrl+Shift, Ctrl+Alt+Shift, Right vs Left)
- Display formatting
- Empty state handling

### Key Events (10 tests)
- VirtualKeyCode correctness (0x41='A', 0x1B=ESC, 0x26=UP)
- VirtualScanCode tracking
- Char/UnshiftedChar fields
- Key down/up states
- Repeat count
- Ctrl+A, Shift+Up, Alt+F4 combinations
- Enhanced key flag

### All Event Types (6 tests)
- Mouse events (position, buttons, flags)
- Focus events (IN/OUT)
- Paste events (START/END)
- Resize events
- Far2l extension events

### Win32 Compatibility (7 tests)
- VK constants: VK_A=0x41, VK_RETURN=0x0D, VK_ESCAPE=0x1B, VK_UP=0x26, etc.
- Virtual/Scan code pairing
- Input source tracking (unix_raw, crossterm, etc.)
- Legacy flag for protocols without explicit KeyUp

### Display Format (4 tests)
- Output contains all required fields
- Modifiers visible in output
- Source tracking for debugging

---

## Interactive Testing Checklist (READY FOR EXECUTION)

See **docs/T-03-VALIDATION.md** for complete details.

### Quick Test
```bash
cargo run --release -- --key-test
```

### What to Verify
1. **Basic Keys**: 'a' → VK:0x0041, Enter → VK:0x000D, Escape → VK:0x001B
2. **Modifiers**: Ctrl+A → "Mods:Ctrl", Shift+Up → "Mods:Shift,Enhanced"
3. **Function Keys**: F1 → VK:0x0070, F4 → VK:0x0073
4. **Arrows with Modifiers**: Shift+Up → VK:0x0026 Mods:Shift
5. **CRITICAL - Special Sequences**:
   - APC far2l (ESC _ f2l ... BEL) → should NOT be dropped, visible as hex
   - Kitty protocol (CSI <code>:<mods>u) → should pass through
   - ANSI sequences (bracketed paste, SGR mouse) → should be intact

---

## Why This Matters

1. **Far2l Integration**: Uses exact format that far2l uses internally
2. **Kitty Protocol**: Converts seamlessly to/from kitty keyboard protocol  
3. **Proven & Stable**: Multi-year field testing across f4, far2l, and multiple terminal clients
4. **Cross-Platform**: Single format handles Windows (native), Unix (all protocols), macOS
5. **Future-Ready**: Far2l APC sequences route through Far2lEventType without collision

---

## Known Limitations (Pre-Validation)

1. **Sequence Parsing Not Yet Implemented** (T-04)
   - InputEvent format is defined and validated
   - Decoders (ANSI, kitty, far2l APC, win32-input-mode) will be implemented in T-04
   - Current: sequences visible as raw bytes in m5 --key-test
   - Post-T-04: sequences decode to proper InputEvent objects

2. **Interactive Testing Required**
   - Unit tests validate format correctness
   - Full validation requires actual terminal interaction
   - Different terminals may have different protocol support

3. **Platform-Specific**
   - Unix: raw stdin read (passes everything through)
   - Windows: crossterm::event (may filter some sequences)
   - macOS: similar to Unix

---

## Next Steps

1. **Interactive Validation** (owner's responsibility)
   - Run `cargo run --release -- --key-test` in various terminals
   - Verify key combinations and special sequences
   - Confirm no sequences are dropped

2. **T-04: Implement Decoders**
   - Legacy xterm decoder (ANSI sequences)
   - Kitty keyboard protocol decoder
   - Win32-input-mode decoder  
   - Far2l APC decoder (binary sequences)
   - SGR mouse decoder
   - Bracketed paste detector

3. **Integration Testing**
   - Test with actual file manager UI
   - Verify keymap lookups work
   - Test action dispatching based on InputEvent

---

## Files Changed

- `crates/m5-term/src/key.rs` — InputEvent types + 35+ tests
- `crates/m5-term/src/key_test.rs` — Interactive test mode  
- `crates/m5-term/src/lib.rs` — Type exports
- `docs/T-03-VALIDATION.md` — Complete validation plan
- `docs/DECISIONS.md` — D-03 updated (Win32 format)
- `docs/PROGRESS.md` — Status updated

---

## Run Tests

```bash
# Unit tests
cargo test -p m5-term --lib key::tests

# Interactive key test
cargo run --release -- --key-test

# All tests (CI)
cargo test --all
```

---

## References

- **Win32 INPUT_RECORD**: https://docs.microsoft.com/windows/console/input-record-str
- **Virtual-Key Codes**: https://docs.microsoft.com/windows/win32/inputdev/virtual-key-codes
- **unxed/winkeys**: https://github.com/unxed/winkeys (reference implementation)
- **unxed/vtinput**: https://github.com/unxed/vtinput (far2l integration)
- **DESIGN.md D-03**: Architecture decision documentation

---

## Architecture Decision

✅ **D-03: Win32-compatible InputEvent format**

**Rationale:**
- Proven format used by far2l, f4, and major terminal clients
- Seamless kitty protocol conversion
- Forward-compatible with all input protocols (ANSI, kitty, far2l APC, win32-input-mode)
- Single universal format reduces complexity

**Implementation:**
- m5-term/src/key.rs (InputEvent struct with all fields)
- Full Win32 ControlKeyState bitflag support
- All event types supported (Key, Mouse, Focus, Paste, Far2l, Resize)

**Status:** ✅ Implemented and unit-tested

---

## Validation Summary

| Component | Status | Details |
|-----------|--------|---------|
| InputEvent struct | ✅ | Fully implemented with all fields |
| ControlKeyState flags | ✅ | All 9 Win32 flags supported |
| EventType enum | ✅ | All 6 types (Key, Mouse, Focus, Paste, Far2l, Resize) |
| Display formatting | ✅ | Win32-style: VK:0xXXXX Scan:0xXXXX Char:X Mods:... |
| Unit tests | ✅ | 35+ tests, all passing |
| Interactive test mode | ✅ | m5 --key-test ready to run |
| Special sequence handling | ⏳ | Raw byte pass-through validated; decoding in T-04 |

---

**Ready for:** Owner interactive validation + T-04 (decoder implementation)

**Created:** 2026-10-01  
**Task:** T-03 from DESIGN.md Iteration 0  
**Related:** D-03, T-04
