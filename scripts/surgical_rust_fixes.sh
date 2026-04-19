#!/bin/bash
# SexOS SASOS v1.0.0 - Surgical Rust Fixes
set -euo pipefail

PROJECT="$PWD"
cd "$PROJECT" || { echo "ERR: no $PROJECT"; exit 1; }

echo "--> Applying surgical fixes to kernel source..."

# 1. Fix E0521: Lifetime escape in memory.rs
sed -i.bak "s/pub fn init(mmap: &limine::request::MemmapResponse/pub fn init(mmap: \&'static limine::request::MemmapResponse/" kernel/src/memory.rs

# 2. Fix E0793: Unaligned packed struct reference in apic.rs
sed -i.bak "s/region.pci_segment_group, region.bus_number_start, region.bus_number_end, region.base_address);/{region.pci_segment_group}, {region.bus_number_start}, {region.bus_number_end}, {region.base_address});/" kernel/src/apic.rs

# 3. Silence unused variable 'r8' in interrupts.rs
sed -i.bak "s/r8: u64) -> u64/_r8: u64) -> u64/" kernel/src/interrupts.rs

# 4. Silence unused variable 'mcfg' in hw/init.rs
sed -i.bak "s/if let Some(mcfg)/if let Some(_mcfg)/" kernel/src/hw/init.rs

# 5. Silence unused import in smp.rs
sed -i.bak "s/use core::sync::atomic::Ordering;//" kernel/src/smp.rs

# 6. Silence unused import in hw/init.rs
sed -i.bak "s/use x86_64::PhysAddr;//" kernel/src/hw/init.rs

# 7. Silence unused imports in memory/allocator.rs and pku.rs
# Targeting the exact line to only leave structures::paging::*
sed -i.bak "s/use x86_64::{VirtAddr, PhysAddr, structures::paging::\*};/use x86_64::structures::paging::\*;/g" kernel/src/memory/allocator.rs
sed -i.bak "s/use x86_64::{VirtAddr, PhysAddr, structures::paging::\*};/use x86_64::structures::paging::\*;/g" kernel/src/memory/pku.rs

echo "--> Fixes applied safely. Running cargo build to verify..."

rustup run nightly cargo build --target x86_64-sex.json -Z build-std=core,alloc --release

echo "--> If build succeeds, you are clear to run ./scripts/safe_sasos_handoff.sh"
