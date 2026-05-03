#!/usr/bin/env bash
# audit_invariant_gates.sh — Pre-commit gate for SexOS architecture invariants.
#
# Checks staged (--cached) diff if files are staged, otherwise working-tree diff.
# Fail-closed on forbidden patterns: kernel edits, sex-pdx edits, framebuffer
# writes outside sexdisplay, shell pixel writes, std/POSIX imports, >2 domains,
# backing-buffer redesign.
#
# Usage:
#   ./scripts/audit_invariant_gates.sh          # auto-detect staged vs working
#   ./scripts/audit_invariant_gates.sh --cached  # force staged
#   ./scripts/audit_invariant_gates.sh --working # force working-tree

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# ---- detect diff mode ----
MODE=working-tree
DIFF_ARGS=()
if [[ "${1:-}" == "--cached" ]]; then
  MODE=cached
  DIFF_ARGS=(--cached)
elif [[ "${1:-}" == "--working" ]]; then
  MODE=working-tree
  DIFF_ARGS=()
elif ! git diff --cached --quiet 2>/dev/null; then
  MODE=cached
  DIFF_ARGS=(--cached)
fi

echo "[audit.invariant.mode] ${MODE}"

# ---- helpers ----
changed_files() {
  git diff "${DIFF_ARGS[@]}" --name-only
}

changed_rs_files() {
  git diff "${DIFF_ARGS[@]}" --name-only -- '*.rs'
}

diff_content() {
  git diff "${DIFF_ARGS[@]}"
}

_passed=0
_failed=0

pass() {
  _passed=$((_passed + 1))
  echo "[audit.invariant.pass] $1"
}

fail() {
  _failed=$((_failed + 1))
  echo "[audit.invariant.fail] $1"
}

# ---- collect diff ----
CHANGED_FILES="$(changed_files || true)"
CHANGED_RS="$(changed_rs_files || true)"
DIFF="$(diff_content || true)"

# ---- domain counters ----
dom_kernel=0
dom_sexpdx=0
dom_display=0
dom_shell=0
dom_input=0
dom_usb=0
dom_apps=0
dom_other=0

while IFS= read -r f; do
  case "$f" in
    kernel/*)    dom_kernel=1   ;;
    crates/sex-pdx/*) dom_sexpdx=1 ;;
    servers/sexdisplay/*) dom_display=1 ;;
    servers/silk-shell/*) dom_shell=1   ;;
    servers/sexinput/*)   dom_input=1   ;;
    servers/sexusb/*)     dom_usb=1     ;;
    apps/*)       dom_apps=1    ;;
    *)            dom_other=1   ;;
  esac
done <<< "$CHANGED_FILES"

# domain count excludes docs/ and scripts/ (non-feature)
FEATURE_DOMAIN_COUNT=$((dom_kernel + dom_sexpdx + dom_display + dom_shell + dom_input + dom_usb + dom_apps))

# ============================================================
# Gate 1: no kernel edits
# ============================================================
if [ "$dom_kernel" -gt 0 ]; then
  fail "kernel/ changed without STOP FIRST"
  echo "  Changed files:"
  echo "$CHANGED_FILES" | grep '^kernel/' | sed 's/^/    /'
else
  pass "no kernel edits"
fi

# ============================================================
# Gate 2: no sex-pdx edits
# ============================================================
if [ "$dom_sexpdx" -gt 0 ]; then
  fail "crates/sex-pdx/ changed without STOP FIRST"
  echo "  Changed files:"
  echo "$CHANGED_FILES" | grep '^crates/sex-pdx/' | sed 's/^/    /'
else
  pass "no sex-pdx edits"
fi

# ============================================================
# Gate 3: no framebuffer/pixel writes outside sexdisplay
# ============================================================
fb_hit=false
while IFS= read -r f; do
  # skip sexdisplay files themselves
  case "$f" in servers/sexdisplay/*) continue ;; esac
  # skip non-Rust files (markdown, configs, etc.)
  case "$f" in *.rs) ;; *) continue ;; esac
  # check per-file diff for actual framebuffer/pixel write operations
  file_diff="$(git diff "${DIFF_ARGS[@]}" -- "$f" 2>/dev/null || true)"
  if echo "$file_diff" | grep -qE '^\+.*(write_pixel|composite_pixel|draw_pixel|pixel_buffer|fb\.write|framebuffer)' 2>/dev/null; then
    echo "  Suspicious framebuffer pattern in: $f"
    fb_hit=true
  fi
done <<< "$CHANGED_FILES"

if $fb_hit; then
  fail "framebuffer/pixel write detected outside servers/sexdisplay/"
else
  pass "no framebuffer writes outside sexdisplay"
fi

# ============================================================
# Gate 4: no shell pixel/framebuffer writes
# ============================================================
shell_pixel_hit=false
while IFS= read -r f; do
  case "$f" in servers/silk-shell/*.rs) ;; *) continue ;; esac
  file_diff="$(git diff "${DIFF_ARGS[@]}" -- "$f" 2>/dev/null || true)"
  if echo "$file_diff" | grep -E '^\+.*(0x[0-9a-fA-F]{6}|write_pixel|composite_pixel|draw_pixel|framebuffer)' 2>/dev/null | grep -qvE '(log|marker|comment|print|no_std|non_std)' 2>/dev/null; then
    echo "  Suspicious pixel pattern in: $f"
    shell_pixel_hit=true
  fi
done <<< "$CHANGED_FILES"

if $shell_pixel_hit; then
  fail "silk-shell may be writing pixels or framebuffer"
else
  pass "no shell pixel/framebuffer writes"
fi

# ============================================================
# Gate 5: no std/libc/thread/POSIX in changed Rust files
# ============================================================
std_hit=false
while IFS= read -r f; do
  file_diff="$(git diff "${DIFF_ARGS[@]}" -- "$f" 2>/dev/null || true)"
  # check added lines for `use std::` (but not `use core::` or `extern crate alloc`)
  if echo "$file_diff" | grep -E '^\+.*\buse\s+std\b' | grep -qvE '(extern crate alloc|core|no_std|non_std)' 2>/dev/null; then
    echo "  std import in: $f"
    std_hit=true
  fi
  # check for libc (word-boundaried, not substring in comments like "std/libc/...")
  if echo "$file_diff" | grep -E '^\+.*\blibc\b' 2>/dev/null; then
    echo "  libc reference in: $f"
    std_hit=true
  fi
  # check for thread::sleep, std::time, std::thread on added lines
  if echo "$file_diff" | grep -E '^\+.*\b(thread::sleep|std::time|std::thread)\b' 2>/dev/null; then
    echo "  POSIX/thread/time in: $f"
    std_hit=true
  fi
done <<< "$CHANGED_RS"

if $std_hit; then
  fail "std/libc/thread/POSIX assumptions detected"
else
  pass "no std/libc/thread/POSIX assumptions"
fi

# ============================================================
# Gate 6: no broad multi-domain patch (>2 feature domains)
# ============================================================
if [ "$FEATURE_DOMAIN_COUNT" -gt 2 ]; then
  fail "patch spans $FEATURE_DOMAIN_COUNT feature domains (max 2 without STOP FIRST)"
  echo "  Feature domains: kernel=$dom_kernel sex-pdx=$dom_sexpdx sexdisplay=$dom_display"
  echo "    silk-shell=$dom_shell sexinput=$dom_input sexusb=$dom_usb apps=$dom_apps"
else
  pass "domain count=$FEATURE_DOMAIN_COUNT <= 2"
fi

# ============================================================
# Gate 7: no shared-memory/backing-buffer redesign
# ============================================================
# ============================================================
# Gate 7: no shared-memory/backing-buffer redesign in Rust code
# ============================================================
shm_hit=false
while IFS= read -r f; do
  file_diff="$(git diff "${DIFF_ARGS[@]}" -- "$f" 2>/dev/null || true)"
  if echo "$file_diff" | grep -qiE '^\+.*\b(shared.buffer|backing.buffer|shm_|mmap_fb|zero.copy.fb|shared_memory)\b' 2>/dev/null; then
    echo "  Suspicious pattern in: $f"
    shm_hit=true
  fi
done <<< "$CHANGED_RS"

if $shm_hit; then
  fail "shared-memory/backing-buffer redesign detected"
else
  pass "no backing-buffer redesign"
fi

# ============================================================
# Gate 8: summary
# ============================================================
echo "[audit.invariant.summary] pass=$_passed fail=$_failed"
if [ "$_failed" -gt 0 ]; then
  echo "[audit.invariant.result] FAIL — review violations and STOP FIRST if needed"
  exit 1
fi
echo "[audit.invariant.result] PASS"
