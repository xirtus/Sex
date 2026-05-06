//! RamFS proof module — built-in contract validation.
//!
//! Activated at compile time with `SEXFILES_RAMFS_PROOF=1`.
//! Runs contract conformance tests at startup and emits serial markers.
//! No runtime cost when disabled (const bool folding).
//!
//! Pattern: matches silk-shell SCENE_SETTINGS_PROTOCOL_PROOF.
//!
//! NAMESPACE_CAPS_V1: All backend calls use `caller_pd = 0` (server-internal)
//! which bypasses ownership checks. Proof runs before any external messages.

extern crate alloc;
use alloc::vec::Vec;
use alloc::format;

use crate::backends::FsBackend;
use crate::messages;
use crate::pdx::serial_println;
use crate::vfs;

/// Server-internal PD value: bypasses owner checks.
const SELF_PD: u32 = 0;

/// Run all RamFS proof checks.
/// Called from trampoline_main when SEXFILES_RAMFS_PROOF is set.
pub fn run_all_proofs() {
    serial_println!("[sexfiles.ramfs.proof.start]");

    // ── Proof 1: Create/write/read roundtrip ──
    proof_create_write_read();

    // ── Proof 2: Invalid handle rejected ──
    proof_invalid_handle();

    // ── Proof 3: Oversized name rejected ──
    proof_oversized_name();

    // ── Proof 4: Out-of-bounds write rejected ──
    proof_oob_write();

    // ── Proof 5: Out-of-bounds read clamped ──
    proof_oob_read_clamped();

    // ── Proof 6: Max files limit enforced ──
    proof_max_files();

    // ── Proof 7: Close+reopen by name (data persists) ──
    proof_close_reopen_persist();

    // ── Proof 8: Non-owner denied ──
    proof_non_owner_denied();

    serial_println!("[sexfiles.ramfs.proof.done] ALL CHECKS PASSED");
}

fn proof_create_write_read() {
    let name = b"proof_roundtrip";
    let handle = vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, SELF_PD)
        .expect("[proof.1] open failed");
    assert!(handle > 0, "[proof.1] handle > 0");

    let data = b"RamFS Contract Lock V1";
    let written = vfs::RAMFS.write(handle, 0, data, SELF_PD)
        .expect("[proof.1] write failed");
    assert_eq!(written as usize, data.len(), "[proof.1] write len");

    let mut buf = [0u8; 64];
    let n = vfs::RAMFS.read(handle, 0, &mut buf, SELF_PD)
        .expect("[proof.1] read failed");
    assert_eq!(&buf[..n as usize], data, "[proof.1] data roundtrip");

    vfs::RAMFS.close(handle, SELF_PD).expect("[proof.1] close failed");
    serial_println!("[sexfiles.ramfs.proof.1] create/write/read roundtrip OK");
}

fn proof_invalid_handle() {
    let mut buf = [0u8; 8];
    let result = vfs::RAMFS.read(0xBAD, 0, &mut buf, SELF_PD);
    assert!(result.is_err(), "[proof.2] bad handle must err");
    assert_eq!(result.unwrap_err(), messages::ERR_INVALID_HANDLE, "[proof.2] err code");

    let result = vfs::RAMFS.close(0xDEAD, SELF_PD);
    assert!(result.is_err(), "[proof.2] bad close must err");
    assert_eq!(result.unwrap_err(), messages::ERR_INVALID_HANDLE, "[proof.2] close err code");

    serial_println!("[sexfiles.ramfs.proof.2] invalid handle rejected OK");
}

fn proof_oversized_name() {
    let long = b"this_name_exceeds_the_24_byte_maximum_for_ramfs";
    let result = vfs::RAMFS.open(long, messages::RAMFS_O_CREATE, 0, SELF_PD);
    assert!(result.is_err(), "[proof.3] long name must err");
    assert_eq!(result.unwrap_err(), messages::ERR_NAME_TOO_LONG, "[proof.3] err code");

    serial_println!("[sexfiles.ramfs.proof.3] oversized name rejected OK");
}

fn proof_oob_write() {
    let name = b"proof_oob";
    let handle = vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, SELF_PD)
        .expect("[proof.4] open failed");

    let big = [0xBBu8; messages::RAMFS_MAX_FILE_SIZE + 1];
    let result = vfs::RAMFS.write(handle, 0, &big, SELF_PD);
    assert!(result.is_err(), "[proof.4] overflow write must err");
    assert_eq!(result.unwrap_err(), messages::ERR_OVERFLOW, "[proof.4] err code");

    // Write at max boundary should succeed
    let ok_data = [0xCCu8; messages::RAMFS_MAX_FILE_SIZE];
    let written = vfs::RAMFS.write(handle, 0, &ok_data, SELF_PD)
        .expect("[proof.4] boundary write should succeed");
    assert_eq!(written as usize, messages::RAMFS_MAX_FILE_SIZE, "[proof.4] boundary write len");

    // Write beyond max should fail
    let result = vfs::RAMFS.write(handle, 1, &ok_data, SELF_PD);
    assert!(result.is_err(), "[proof.4] post-boundary write must err");

    vfs::RAMFS.close(handle, SELF_PD).expect("[proof.4] close failed");
    serial_println!("[sexfiles.ramfs.proof.4] OOB write rejected OK");
}

fn proof_oob_read_clamped() {
    let name = b"proof_oob_read";
    let handle = vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, SELF_PD)
        .expect("[proof.5] open failed");

    // Write 10 bytes
    let data = b"1234567890";
    vfs::RAMFS.write(handle, 0, data, SELF_PD).expect("[proof.5] write failed");

    // Read beyond EOF returns 0 bytes (not error)
    let mut buf = [0u8; 8];
    let n = vfs::RAMFS.read(handle, 100, &mut buf, SELF_PD)
        .expect("[proof.5] OOB read should not err");
    assert_eq!(n, 0, "[proof.5] OOB read returns 0");

    // Partial read at boundary
    let n = vfs::RAMFS.read(handle, 5, &mut buf, SELF_PD).expect("[proof.5] partial read");
    assert_eq!(n, 5, "[proof.5] partial read returns remaining");
    assert_eq!(&buf[..5], b"67890", "[proof.5] partial read data");

    vfs::RAMFS.close(handle, SELF_PD).expect("[proof.5] close failed");
    serial_println!("[sexfiles.ramfs.proof.5] OOB read clamped OK");
}

fn proof_max_files() {
    // Create max files
    let mut handles = Vec::new();
    for i in 0..messages::RAMFS_MAX_FILES {
        let name = format!("file_{}", i);
        let h = vfs::RAMFS.open(name.as_bytes(), messages::RAMFS_O_CREATE, 0, SELF_PD)
            .expect("[proof.6] max file create");
        handles.push(h);
    }

    // Next create should fail with ERR_FULL
    let result = vfs::RAMFS.open(b"file_overflow", messages::RAMFS_O_CREATE, 0, SELF_PD);
    assert!(result.is_err(), "[proof.6] over max must err");
    assert_eq!(result.unwrap_err(), messages::ERR_FULL, "[proof.6] err code");

    // Cleanup
    for h in handles {
        vfs::RAMFS.close(h, SELF_PD).expect("[proof.6] close failed");
    }
    assert_eq!(vfs::RAMFS.len(SELF_PD), 0, "[proof.6] all closed");

    serial_println!("[sexfiles.ramfs.proof.6] max files limit enforced OK");
}

fn proof_close_reopen_persist() {
    let name = b"proof_persist";
    let handle = vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, SELF_PD)
        .expect("[proof.7] open failed");

    let data = b"Persist!";
    vfs::RAMFS.write(handle, 0, data, SELF_PD).expect("[proof.7] write failed");

    // Close releases handle but data persists
    vfs::RAMFS.close(handle, SELF_PD).expect("[proof.7] close failed");

    // Reopen by name — should find same data
    let reopened = vfs::RAMFS.open(name, 0, 0, SELF_PD).expect("[proof.7] reopen failed");
    let mut buf = [0u8; 64];
    let n = vfs::RAMFS.read(reopened, 0, &mut buf, SELF_PD).expect("[proof.7] read failed");
    assert_eq!(n as usize, data.len(), "[proof.7] reopen read len");
    assert_eq!(&buf[..n as usize], data, "[proof.7] reopen data intact");

    vfs::RAMFS.close(reopened, SELF_PD).expect("[proof.7] close reopened failed");
    serial_println!("[sexfiles.ramfs.proof.7] close+reopen data persistence OK");
}

/// Proof 8: Non-owner access is denied.
/// Creates a file with PD=1, then attempts access with PD=2; must get ERR_PERM_DENIED.
fn proof_non_owner_denied() {
    let name = b"proof_owner";
    // Create file as "PD 1"
    let handle = vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, 1)
        .expect("[proof.8] open failed");

    // PD 2 should be denied read
    let mut buf = [0u8; 8];
    let result = vfs::RAMFS.read(handle, 0, &mut buf, 2);
    assert!(result.is_err(), "[proof.8] non-owner read must err");
    assert_eq!(result.unwrap_err(), messages::ERR_PERM_DENIED, "[proof.8] err code");

    // PD 2 should be denied write
    let result = vfs::RAMFS.write(handle, 0, b"x", 2);
    assert!(result.is_err(), "[proof.8] non-owner write must err");
    assert_eq!(result.unwrap_err(), messages::ERR_PERM_DENIED, "[proof.8] err code");

    // PD 2 should be denied close
    let result = vfs::RAMFS.close(handle, 2);
    assert!(result.is_err(), "[proof.8] non-owner close must err");
    assert_eq!(result.unwrap_err(), messages::ERR_PERM_DENIED, "[proof.8] err code");

    // PD 2 should be denied stat
    let result = vfs::RAMFS.stat(handle, 2);
    assert!(result.is_err(), "[proof.8] non-owner stat must err");
    assert_eq!(result.unwrap_err(), messages::ERR_PERM_DENIED, "[proof.8] err code");

    // PD 2 should be denied open-by-name (reopen)
    let result = vfs::RAMFS.open(name, 0, 0, 2);
    assert!(result.is_err(), "[proof.8] non-owner open must err");
    assert_eq!(result.unwrap_err(), messages::ERR_PERM_DENIED, "[proof.8] err code");

    // Owner (PD 1) should still have access
    let n = vfs::RAMFS.read(handle, 0, &mut buf, 1)
        .expect("[proof.8] owner read should succeed");
    assert_eq!(n, 0, "[proof.8] owner read empty file");

    // Cleanup as owner
    vfs::RAMFS.close(handle, 1).expect("[proof.8] owner close failed");

    serial_println!("[sexfiles.ramfs.proof.8] non-owner access denied OK");
}
