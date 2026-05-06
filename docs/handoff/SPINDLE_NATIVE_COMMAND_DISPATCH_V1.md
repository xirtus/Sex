# SPINDLE_NATIVE_COMMAND_DISPATCH_V1

**Date:** 2026-05-06
**Status:** Command dispatch proven — 8 built-in commands, simple tokenizer, unknown rejection
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Previous:** SPINDLE_SCROLLBACK_RING_V1
**Next:** SPINDLE_SEXFILES_HISTORY_V1

---

## Summary

Added a native command dispatcher with 8 built-in commands:
- Simple whitespace tokenizer: splits command name from args
- Exact byte-match dispatch (no regex, no glob, no shell metacharacters)
- Unknown commands print "unknown command" (not in dispatch, but recognized as unknown by proof)
- All output through scrollback ring buffer
- `clear` command resets scrollback to fresh state
- `help` lists all 8 commands
- Unavailable commands honestly report status

---

## Command Table

| # | Command | Args | Description | Status |
|---|---------|------|-------------|--------|
| 1 | `help` | — | List built-in commands | **Implemented** |
| 2 | `clear` | — | Reset scrollback | **Implemented** |
| 3 | `status` | — | Spindle V1 status + SexOS info | **Implemented** |
| 4 | `pd` | — | Static PD list (11 PDs) | **Implemented** (live query unavailable) |
| 5 | `servers` | — | Static server list | **Implemented** |
| 6 | `bell` | — | Bell bridge status | **Pending** (honest report) |
| 7 | `files` | — | SexFiles bridge status | **Pending** (honest report) |
| 8 | `launch quil` | `quil` | Request Quil surface | **Unavailable** (honest report) |

All commands are local only — no external process spawning, no PDX calls, no POSIX exec, no shell scripting.

---

## Tokenizer

```
Input:  "launch quil"
          ^cmd^  ^args^
Output: (b"launch", b"quil")

Input:  "  help  "
Output: (b"help", b"")

Input:  ""
Output: (b"", b"") -- no-op dispatch
```

Simple whitespace split. No quoting, no escaping, no metacharacters.

---

## Dispatch Logic

```rust
fn dispatch(line: &[u8], sb: &mut Scrollback) -> bool {
    let (cmd, args) = tokenize(line);
    match cmd {
        b"help"   => { push help text to sb; true }
        b"clear"  => { reset sb; true }
        b"status" => { push status lines; true }
        b"pd"     => { push PD list; true }
        b"servers"=> { push server list; true }
        b"bell"   => { push pending status; true }
        b"files"  => { push pending status; true }
        b"launch" => { push unavailable or unknown target; true }
        _         => false  // unknown command
    }
}
```

Returns `true` if command recognized, `false` for unknown commands.

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +133 lines — tokenizer, dispatch, 8 handlers, proof stages 10-17 |
| `docs/handoff/SPINDLE_NATIVE_COMMAND_DISPATCH_V1.md` | NEW |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST |
| `crates/sex-pdx/` | No ABI changes |
| `servers/silk-shell/` | No launch routing |
| `servers/sexfiles/` | No storage calls |

---

## Proof Gate (Extended to 17 Stages)

### New Stages (10-17): Command Dispatch

| Stage | Command | Assertion | Marker |
|-------|---------|-----------|--------|
| 10 | `help` | Recognized | `[spindle.cmd.dispatch] cmd=help` |
| 11 | `status` | Recognized | `[spindle.cmd.dispatch] cmd=status` |
| 12 | `clear` | Scrollback reset (lines before > after) | `[spindle.cmd.clear]` |
| 13 | `pd` | Recognized | `[spindle.cmd.dispatch] cmd=pd` |
| 14 | `servers` | Recognized | `[spindle.cmd.dispatch] cmd=servers` |
| 15 | `asdf` | **NOT** recognized (unknown) | `[spindle.cmd.unknown] cmd=asdf` |
| 16 | `bell` | Recognized (pending report) | `[spindle.cmd.dispatch] cmd=bell` |
| 17 | `launch quil` | Recognized (unavailable report) | `[spindle.cmd.launch_quil.unavailable]` |

### Updated Stage 5 (Enter)

Now calls `dispatch()` on the command line. Pushes command echo to scrollback, then dispatches. "test" is tested as an unknown command (dispatch returns false).

---

## Unavailable Commands (Honest Reporting)

| Command | Why Unavailable |
|---------|-----------------|
| `bell` | Spindle not kernel-spawned — no PDX call to sexbell |
| `files` | Spindle not kernel-spawned — no PDX call to sexfiles |
| `launch quil` | Spindle not kernel-spawned — no PDX call to silk-shell |

All three require: kernel spawn (STOP FIRST), PDX slot registration, and silk-shell routing.

---

## Forbidden Scan

```
rg -n "pty|bash|/bin/sh|std::process|Command::|libc|pthread|thread::spawn" apps/spindle/
```
Result: **0 matches** — CLEAN

No POSIX, no shell, no host command execution, no external processes.

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS (4 warnings) |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

---

## Next Prompt

```
SPINDLE_SEXFILES_HISTORY_V1
```

Adds: command history ring (128 entries), Arrow Up/Down navigation, SexFiles RamFS persistence, proof stages for history save/load.

---

## Contract Boundaries Preserved

- **No POSIX shell** — no PATH, no env vars, no shell scripting, no PTY
- **No host commands** — all handlers are local Rust functions
- **No process model** — no fork/exec/spawn
- **No pipes/redirect** — cmd output goes directly to scrollback
- **No dynamic loading** — commands are compile-time match arms
- **Bounded output** — all output lines ≤ 80 chars, pushed to bounded ring
