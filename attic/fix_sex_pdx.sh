#!/usr/bin/env bash
set -e

echo "============================================================"
echo "[*] SEX PDX FIX + ABI CLEANUP"
echo "============================================================"

CRATE_DIR="crates/sex-pdx"
LIB_FILE="$CRATE_DIR/src/lib.rs"
CARGO_FILE="$CRATE_DIR/Cargo.toml"

if [ ! -f "$LIB_FILE" ]; then
    echo "[-] Error: Could not find $LIB_FILE"
    exit 1
fi

echo "[*] STEP 1: Patch Cargo.toml (add serde feature)"

if ! grep -q "\[features\]" "$CARGO_FILE"; then
cat >> "$CARGO_FILE" << 'EOT'

[features]
default = []
serde = ["dep:serde"]

[dependencies]
serde = { version = "1", optional = true, features = ["derive"] }
EOT
    echo "[+] Serde features injected."
else
    echo "[!] Features section exists, skipping append (manual check recommended)."
fi

echo "[*] STEP 2: Patch syscall listener to Option B pattern"

# Using Perl to safely swap the monolithic unsafe block for the lateout ABI pattern
perl -0777 -pi -e 's|
let\s+req\s*=\s*PdxRequest::default\(\);\s*
unsafe\s*\{.*?syscall.*?\}|
let mut caller_pd: u64 = 0;
let mut num: u64 = 0;
let mut arg0: u64 = 0;
let mut arg1: u64 = 0;
let mut arg2: u64 = 0;

unsafe {
    core::arch::asm!(
        "syscall",
        in("rax") 28,
        in("rdi") flags,
        lateout("rax") _,
        lateout("rdi") caller_pd,
        lateout("rsi") num,
        lateout("rdx") arg0,
        lateout("rcx") arg1,
        lateout("r8")  arg2,
    );
}

let req = PdxRequest {
    caller_pd,
    num,
    arg0,
    arg1,
    arg2,
};|gs' "$LIB_FILE"

echo "[+] ABI lateout pattern applied."

echo "[*] STEP 3: Ensure struct is sane for SASOS"

# Mac-safe Perl replacement to add #[repr(C)] if it doesn't already exist
perl -0777 -pi -e 's/(?<!#\[repr\(C\)\]\n)(pub\s+)?struct\s+PdxRequest/#[repr(C)]\n${1}struct PdxRequest/g' "$LIB_FILE"

echo "[+] ABI stability (repr C) enforced."

echo "[*] STEP 4: Triggering Clean Release Build"

# Pointing back to the native compilation pipeline
./scripts/clean_build.sh

echo "============================================================"
echo "[+] DONE: PDX ABI CLEAN, BUILD COMPLETE"
echo "============================================================"
