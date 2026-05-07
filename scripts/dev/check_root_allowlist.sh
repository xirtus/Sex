#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

strict=0
if [[ "${1:-}" == "--strict" ]]; then
  strict=1
fi

# Allowlist is intentionally conservative for migration: only top-level files are checked.
# Directories are ignored here.
allowlist=(
  "AGENTS.md" "ARCHITECTURE.md" "CLAUDE.md" "HANDOFF.md" "HEARTBEAT.md" "IDENTITY.md" "LICENSE"
  "Cargo.toml" "Cargo.lock" "Makefile" "rust-toolchain.toml" "x86_64-sex.json"
  "limine.cfg" "sexos_build_spec.toml" "sexos_contract.toml"
  "dev.sh" "build_payload.sh"
  "boot_phase20.sh" "bx.sh" "qemuX-autotest.sh" "qemuX-kbd-only.sh" "qemuX-kbd.sh" "qemuX-ps2.sh" "qemuX.sh" "run_qemu.sh" "sexos_final_boss.sh" "surgical_finality_v2.sh" "verify_fix.sh"
)

declare -A ok
for f in "${allowlist[@]}"; do
  ok["$f"]=1
done

unknown=()
while IFS= read -r name; do
  if [[ -z "${ok[$name]:-}" ]]; then
    unknown+=("$name")
  fi
done < <(find . -maxdepth 1 -type f -printf '%f\n' | sort)

if [[ ${#unknown[@]} -eq 0 ]]; then
  echo "root-allowlist: clean"
  exit 0
fi

echo "root-allowlist: found ${#unknown[@]} unallowlisted root files"
printf '%s\n' "${unknown[@]}"

if [[ $strict -eq 1 ]]; then
  exit 1
fi
