#!/usr/bin/env bash
set -euo pipefail

BUILD_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/entrypoint_build.sh"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
SPEC_PATH="$ROOT_DIR/sexos_build_spec.toml"

if [[ "$#" -ne 0 ]]; then
  echo "[FAIL] entrypoint takes no mode flags. deterministic single trace only."
  exit 1
fi

fail() {
  echo "[FAIL] $1"
  exit 1
}

validate_contract() {
  local contract="sexos_contract.toml"
  [[ -f "$contract" ]] || fail "missing contract: $contract"
  rg -n "^\[contract\]" "$contract" >/dev/null || fail "contract section missing"
  rg -n "^\[ipc\]" "$contract" >/dev/null || fail "ipc section missing"
  rg -n "transport\s*=\s*\"register-only\"" "$contract" >/dev/null || fail "ipc transport must be register-only"
  rg -n "legacy_struct_return\s*=\s*false" "$contract" >/dev/null || fail "legacy_struct_return must be false"
  rg -n "legacy_r9_path\s*=\s*false" "$contract" >/dev/null || fail "legacy_r9_path must be false"
  rg -n "^\[ipc\\.pdx_call\]" "$contract" >/dev/null || fail "ipc.pdx_call section missing"
  rg -n "status_reg\s*=\s*\"rax\"" "$contract" >/dev/null || fail "pdx_call status_reg must be rax"
  rg -n "value_reg\s*=\s*\"rsi\"" "$contract" >/dev/null || fail "pdx_call value_reg must be rsi"
}

validate_silk_de_gates() {
  rg -n "pub fn validate_deterministic_vectors\\(" crates/silkbar-model/src/lib.rs >/dev/null \
    || fail "silkbar-model missing validate_deterministic_vectors"
  rg -n "validate_deterministic_vectors\\(\\)" servers/silkbar/src/main.rs >/dev/null \
    || fail "silkbar startup must enforce deterministic vectors gate"
  rg -n "validate_deterministic_vectors\\(\\)" servers/sexdisplay/src/main.rs >/dev/null \
    || fail "sexdisplay startup must enforce deterministic vectors gate"
}

spec_get() {
  local key="$1"
  rg -n "^${key}\\s*=\\s*\"[^\"]+\"" "$SPEC_PATH" | head -n1 | sed -E 's/.*=\s*"([^"]+)".*/\1/'
}

echo "[SEXOS ENTRYPOINT] BUILD_ROOT=$BUILD_ROOT"
[[ -f "$SPEC_PATH" ]] || fail "missing build spec: $SPEC_PATH"

# 1) ABI guard first, always.
./scripts/sexos_pipeline.sh

# 2) Contract validation before any compile/package/run step, then freeze snapshot.
validate_contract
validate_silk_de_gates

expected_contract_hash="$(spec_get contract_sha256)"
actual_contract_hash="$(sha256sum sexos_contract.toml | awk '{print $1}')"
[[ "$expected_contract_hash" = "$actual_contract_hash" ]] || fail "contract hash mismatch vs spec"

expected_abi_hash="$(spec_get abi_version_hash)"
actual_abi_hash="$({ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } | sha256sum | awk '{print $1}')"
[[ "$expected_abi_hash" = "$actual_abi_hash" ]] || fail "abi_version_hash mismatch vs spec"

SNAP_DIR=".sexos_snapshot"
mkdir -p "$SNAP_DIR"
rm -f "$SNAP_DIR/contract.snapshot.toml" "$SNAP_DIR/abi.snapshot.lock"
cp sexos_contract.toml "$SNAP_DIR/contract.snapshot.toml"
chmod 444 "$SNAP_DIR/contract.snapshot.toml"

# Freeze ABI snapshot once (entrypoint only).
{
  echo "build_root=$BUILD_ROOT"
  sha256sum kernel/src/syscalls/mod.rs
  sha256sum crates/sex-pdx/src/lib.rs
  sha256sum sexos_contract.toml
} > "$SNAP_DIR/abi.snapshot.lock"
chmod 444 "$SNAP_DIR/abi.snapshot.lock"

# 3) Seal environment and execute single linear trace.
export SEXOS_ENTRYPOINT_ACTIVE=1
export SEXOS_TRACE_ACTIVE=1
export SEXOS_BUILD_ROOT="$BUILD_ROOT"
export SEXOS_CONTRACT_SNAPSHOT="$ROOT_DIR/$SNAP_DIR/contract.snapshot.toml"
export SEXOS_ABI_SNAPSHOT="$ROOT_DIR/$SNAP_DIR/abi.snapshot.lock"
export SEXOS_BUILD_SPEC="$SPEC_PATH"

./scripts/sexos_build_trace.sh "$SPEC_PATH"

echo "[SEXOS ENTRYPOINT] success"
