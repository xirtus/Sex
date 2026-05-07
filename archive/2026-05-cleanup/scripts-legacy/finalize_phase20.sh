#!/usr/bin/env bash

# Exit immediately if a command fails
set -e

echo "[*] INITIATING PHASE 20 FINAL INTEGRATION"

echo ">>> Patching Makefile with Docker and Boot targets..."

# We use printf to guarantee literal tab characters (\t) are written to the Makefile
# This prevents the classic "missing separator" Make error caused by copy-pasting spaces.

printf "\n# ==========================================\n" >> Makefile
printf "# PHASE 20 AUTOMATION TARGETS\n" >> Makefile
printf "# ==========================================\n\n" >> Makefile

# --- release target ---
printf "release:\n" >> Makefile
printf "\t@echo \"[*] Compiling Kernel & Userland Payload...\"\n" >> Makefile
printf "\t./build_payload.sh\n" >> Makefile
printf "\t@echo \"[*] Building ISO...\"\n" >> Makefile
printf "\t@# If your ISO target is named differently, adjust the line below:\n" >> Makefile
printf "\t\$(MAKE) iso\n\n" >> Makefile

# --- run-sasos target ---
printf "run-sasos:\n" >> Makefile
printf "\t@echo \"[*] Booting SASOS Hardware Isolation Substrate...\"\n" >> Makefile
printf "\tqemu-system-x86_64 \\\\\n" >> Makefile
printf "\t\t-M q35 \\\\\n" >> Makefile
printf "\t\t-m 512M \\\\\n" >> Makefile
printf "\t\t-cpu max,+pku \\\\\n" >> Makefile
printf "\t\t-cdrom sexos.iso \\\\\n" >> Makefile
printf "\t\t-serial stdio \\\\\n" >> Makefile
printf "\t\t-boot d\n\n" >> Makefile

echo "[*] Makefile successfully patched."
echo "[*] Launching containerized build pipeline and booting QEMU..."

# Execute the final pipeline
./scripts/clean_build.sh && make run-sasos

