# T-03 Validation Report: Win32 InputEvent Format

**Task**: T-03 (Spike) - Validate Win32-compatible InputEvent format for keyboard input  
**Date**: 2026-10-01  
**Owner Directive**: Validate architecture using m5 --key-test  
**Status**: ✓ UNIT TESTS PASSING | ⏳ AWAITING INTERACTIVE VALIDATION

---

## Implementation Summary

### What Was Implemented

**File: `crates/m5-term/src/key.rs`**
- `InputEvent` struct: Universal event container (Win32 INPUT_RECORD compatible)
- `ControlKeyState`: Modifier flags matching Win32 dwControlKeyState
- `EventType` enum: Key, Mouse, Focus, Paste, Far2l, Resize
- Full Display implementation for debugging
- Constructor methods: `key()`, `mouse()`, `focus()`, `paste()`, `resize()`

**File: `crates/m5-term/src/key_test.rs`**
- Interactive `m5 --key-test` mode
- Unix: raw stdin input + InputEvent display
- Windows: crossterm event → InputEvent conversion
- Validates all sequences pass through (not dropped)

---

## Unit Test Coverage (PASSING ✓)

**35+ tests implemented, covering:**

1. **ControlKeyState Validation** (8 tests)
   - Individual flags: LEFT_CTRL, RIGHT_CTRL, LEFT_ALT, RIGHT_ALT, SHIFT, ENHANCED
   - Combinations: Ctrl+Shift, Ctrl+Alt+Shift, Right vs. Left
   - Display formatting
   - Empty state (None)

2. **Key Event Tests** (10 tests)
   - VirtualKeyCode correctness (0x41 for 'A', 0x1B for ESC, 0x26 for UP)
   - VirtualScanCode tracking
   - Char/UnshiftedChar fields
   - Key down/up states
   - Repeat count handling
   - Ctrl+A, Shift+Up, Alt+F4 combinations
   - Enhanced key flag

3. **All Event Types** (6 tests)
   - Mouse events (position, buttons, flags)
   - Focus events (IN/OUT)
   - Paste events (START/END)
   - Resize events
   - Far2l extension events

4. **Win32 Compatibility** (7 tests)
   - Common VK constants (VK_A=0x41, VK_RETURN=0x0D, VK_ESCAPE=0x1B, VK_UP=0x26, etc.)
   - Virtual/Scan code pairing
   - Input source tracking (unix_raw, crossterm, etc.)
   - Legacy flag for KeyUp-less protocols

5. **Display Format Tests** (4 tests)
   - Output contains all required fields
   - Modifiers visible: "Ctrl", "Alt", "Shift"
   - Source tracking visible for debugging

---

## Interactive Testing Checklist (PENDING)

### Prerequisites
- Terminal supporting various input protocols (xterm, tmux, kitty, far2l if available)
- Ability to send special key combinations

### Test Plan

Run: `cargo run --release -- --key-test`

#### 1. Basic Key Input
- [ ] Press 'a' → should display `Key{VK:0x0041 Scan:0x1E Char:'a' DOWN Mods:None}`
- [ ] Press 'A' (Shift+A) → should display `Key{... Mods:Shift}`
- [ ] Press Enter → should display `Key{VK:0x000D ...}`
- [ ] Press Escape → should display `Key{VK:0x001B ...}`

#### 2. Modifier Combinations
- [ ] Ctrl+A → `Mods:Ctrl`
- [ ] Shift+A → `Mods:Shift`
- [ ] Alt+A → `Mods:Alt`
- [ ] Ctrl+Shift+A → `Mods:Ctrl,Shift`
- [ ] Ctrl+Alt+Shift+A → `Mods:Ctrl,Alt,Shift`

#### 3. Function Keys
- [ ] F1 → `VK:0x0070`
- [ ] F4 → `VK:0x0073`
- [ ] Shift+F1 → `VK:0x0070 Mods:Shift`
- [ ] Ctrl+F5 → `VK:0x0074 Mods:Ctrl`

#### 4. Arrow Keys
- [ ] Up → `VK:0x0026 Mods:None`
- [ ] Down → `VK:0x0028`
- [ ] Left → `VK:0x0025`
- [ ] Right → `VK:0x0027`
- [ ] Shift+Up → `VK:0x0026 Mods:Shift`

#### 5. Special Sequences (CRITICAL FOR FAR2L/KITTY)
- [ ] APC far2l sequence (if terminal supports)
  - Should NOT be lost/dropped
  - Should display as raw bytes if not parsed
  - Example: `ESC _ f2l ... BEL` or `ESC _ f2l ... ESC \`
  - Validation: Output shows full sequence hex

- [ ] Kitty keyboard protocol (if enabled)
  - CSI <code>:<alt>;<mods>:<event>u format
  - Should pass through intact
  - Example: pressing 'a' with Ctrl should send CSI 97:5u
  - Validation: Raw bytes visible in output

- [ ] ANSI sequence pass-through
  - Bracketed paste: `CSI 200~ ... CSI 201~`
  - SGR mouse: `CSI < button;x;y M`
  - Should all pass through intact
  - Validation: Sequences visible as raw bytes

#### 6. Windows-Specific (if on Windows)
- [ ] win32-input-mode (if supported by Windows Terminal)
  - Format: `CSI Vk;Sc;Uc;Kd;Cs;Rc _`
  - Should convert to InputEvent correctly

---

## Validation Criteria (Success = All Pass)

### ✅ Core Format Validation
- [x] InputEvent struct compiles and is documented
- [x] ControlKeyState correctly bitwise combines modifiers
- [x] EventType enum covers all required types
- [x] Display format matches Win32 style (VK:0xXXXX, Mods:...)
- [x] Modifier flags match Win32 constants (0x0001 for RIGHT_ALT, 0x0010 for SHIFT, etc.)

### ✅ Unit Tests
- [x] All 35+ tests pass (run: `cargo test -p m5-term`)
- [x] Key event creation works
- [x] Modifier combinations work
- [x] Display formatting is correct
- [x] Win32 VK constants are properly used

### ⏳ Interactive Tests (Awaiting Execution)
- [ ] `m5 --key-test` displays InputEvent correctly for basic keys
- [ ] Modifiers are tracked (Ctrl, Alt, Shift combinations)
- [ ] Function keys work (F1-F12)
- [ ] Arrow keys work with correct VirtualKeyCode
- [ ] Special sequences pass through (not dropped)
- [ ] APC far2l sequences visible in output
- [ ] Kitty protocol sequences visible (if supported)
- [ ] ANSI sequences pass through intact

---

## Known Limitations (Pre-Validation)

1. **Sequence Parsing Not Yet Implemented** (T-04)
   - Current: InputEvent format is defined, but decoder (T-04) will parse sequences
   - APC/kitty/ANSI sequences visible as raw bytes in m5 --key-test
   - Once T-04 is complete: sequences will decode to proper InputEvent objects

2. **Interactive Terminal Testing Required**
   - Unit tests validate format correctness
   - Full validation requires actual terminal interaction
   - Different terminals may have different protocol support

3. **Platform-Specific Behavior**
   - Unix: raw stdin read (passes everything through)
   - Windows: crossterm::event (may filter some sequences)
   - macOS: similar to Unix

---

## Next Steps

1. **Interactive Testing** (this validation)
   - Run `m5 --key-test` in various terminals
   - Test key combinations and special sequences
   - Verify no sequences are dropped

2. **T-04: Implement Decoders**
   - Legacy xterm decoder (ANSI sequences)
   - Kitty keyboard protocol decoder
   - Win32-input-mode decoder
   - Far2l APC decoder
   - SGR mouse decoder
   - Bracketed paste detector

3. **Integration Testing**
   - Test with actual file manager UI
   - Verify keymap lookups work
   - Test action dispatching

---

## References

- **Win32 INPUT_RECORD**: https://docs.microsoft.com/windows/console/input-record-str
- **Virtual-Key Codes**: https://docs.microsoft.com/windows/win32/inputdev/virtual-key-codes
- **unxed/winkeys**: https://github.com/unxed/winkeys (reference format)
- **unxed/vtinput**: https://github.com/unxed/vtinput (reference implementation)
- **DESIGN.md D-03**: Architecture decision documentation

---

## Test Command

```bash
# Run unit tests
cargo test -p m5-term --lib key::tests

# Run interactive key test
cargo run --bin m5 --release -- --key-test
```

---

## Sign-Off

**Architecture Decision**: D-03 (Win32 InputEvent Format) ✓ APPROVED  
**Unit Tests**: ✓ ALL PASSING  
**Interactive Validation**: ⏳ PENDING (awaiting owner's test execution)

**Ready for**: T-04 (Decoder implementation)

---

*T-03 Validation Document*  
*m5 Project - Iteration 0*  
*2026-10-01*
