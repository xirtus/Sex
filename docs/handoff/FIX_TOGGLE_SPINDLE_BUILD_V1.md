# FIX_TOGGLE_SPINDLE_BUILD_V1

**Status:** Fixed. Full entrypoint build green.
**Date:** 2026-05-05

---

## 1. First Compiler Errors (3 errors in silk-shell)

### Error 1: non-ASCII character in byte string literal
```
error: non-ASCII character in byte string literal
  --> servers/silk-shell/src/main.rs:6722:36
   |
6722 |     yarn_append_output(b"SexOS 0.2 — Spindle terminal");
   |                                    ^ must be ASCII
```

### Error 2: mismatched types in array literal
```
error[E0308]: mismatched types
  --> servers/silk-shell/src/main.rs:6755:35
   |
6755 |     let msg = [b"Active scene: ", &[b'0' + active as u8], ...
     |                                   ^^^^^^^^^^^^^^^^^^^^^^
     |             expected an array with a size of 14, found one with a size of 1
```

### Error 3: mismatched types — array vs slice
```
error[E0308]: mismatched types
  --> servers/silk-shell/src/main.rs:6811:27
   |
6811 |     let args = trim_ascii(cmd);
     |                ---------- ^^^ expected `&[u8]`, found `[u8; 256]`
```

## 2. Root Cause

Silk-shell had 3 pre-existing Rust compilation errors that were latent (never caught because the build pipeline previously stopped earlier for other reasons, or the code was recently added without a clean build cycle):

1. **Em dash in byte string**: `—` (U+2014) is non-ASCII. Byte string literals require ASCII-only content.
2. **Heterogeneous array of byte slices**: `&[u8; N]` has different types for different `N` values. Mixing `&[u8; 14]` and `&[u8; 1]` in the same array is a type mismatch. The array was dead code (line 6757 already had the correct simple output).
3. **Array passed where slice expected**: `cmd` is `[u8; 256]` (from `cmd_buf`), but `trim_ascii()` takes `&[u8]`.

## 3. Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | 3-line fix |

## 4. Exact Fixes

**Fix 1** (line 6722): Replace em dash with ASCII hyphen:
```rust
// Before:
yarn_append_output(b"SexOS 0.2 — Spindle terminal");
// After:
yarn_append_output(b"SexOS 0.2 - Spindle terminal");
```

**Fix 2** (lines 6755-6757): Remove dead heterogeneous array, keep simple output:
```rust
// Before:
    let msg = [b"Active scene: ", &[b'0' + active as u8], b" frames: ", &[b'0' + count]];
    // Simple output for V1.
    yarn_append_output(b"Active scene: 0");
// After:
    // Simple output for V1.
    yarn_append_output(b"Active scene: 0");
```

**Fix 3** (line 6811): Pass slice of used buffer portion:
```rust
// Before:
    let args = trim_ascii(cmd);
// After:
    let args = trim_ascii(&cmd[..len]);
```

## 5. Bell Untouched Confirmation

```bash
rg -n "OP_BELL_LIST|bell\.list\.|kernel\.sexbell\.list" servers/sexbell/src/main.rs kernel/src/init.rs
# → Only Bell list implementation markers (unchanged)
```

No Bell files changed. No kernel/init.rs changes beyond existing list scaffold. No sex-pdx edits.

## 6. Build Result

```
[SEXOS ENTRYPOINT] success
```

Full ISO produced successfully.

## 7. Ready for BELL_LIST_SUMMARY_PROOF_V1

All 3 pre-existing silk-shell build errors fixed. Bell list implementation unchanged. Full build green → QEMU proof may proceed.

---

*End of FIX_TOGGLE_SPINDLE_BUILD_V1.md*
