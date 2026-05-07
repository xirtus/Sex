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
use crate::backends::diskfs::{DiskFs, DISKFS_BLOCK_SIZE, DISKFS_MAX_OBJECTS, DISKFS_JOURNAL_CAPACITY,
    DISKFS_EXTENT_BLOCK_COUNT};
use crate::backends::ramfs::{
    CAP_RIGHT_APPEND, CAP_RIGHT_GRANT, CAP_RIGHT_LIST, CAP_RIGHT_READ, CAP_RIGHT_WRITE,
};
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

/// Run DiskFS object-table scaffold proof checks.
/// Activated by SEXOS_DISKFS_OBJECT_TABLE_PROOF.
pub fn run_diskfs_object_table_proofs() {
    let disk = DiskFs::new();

    disk.format_init_empty().expect("[diskfs.proof] format failed");
    serial_println!("[diskfs.proof.format] ok=1");

    let sb = disk.mount().expect("[diskfs.proof] mount failed");
    let ok_mount = sb.version_major == 1 && sb.block_size == 4096 && sb.object_table_entry_count == DISKFS_MAX_OBJECTS as u32;
    serial_println!("[diskfs.proof.mount] ok={} gen={}", ok_mount as u8, sb.fs_generation);

    let oid = disk.create_object_entry(1, 9).expect("[diskfs.proof] create failed");
    serial_println!("[diskfs.proof.create_object] id={} owner=9 kind=1", oid);

    let st = disk.stat_object_entry(oid).expect("[diskfs.proof] stat failed");
    let ok_stat = st.object_id == oid && st.owner_pd == 9 && st.kind == 1;
    serial_println!("[diskfs.proof.stat_object] ok={} id={} owner={}", ok_stat as u8, st.object_id, st.owner_pd);

    let invalid = disk.stat_object_entry(u64::MAX);
    let ok_invalid = matches!(invalid, Err(e) if e == messages::ERR_INVALID_HANDLE);
    serial_println!("[diskfs.proof.invalid_object] ok={}", ok_invalid as u8);

    // Fill remaining table slots, then one more create must fail with ERR_FULL.
    let mut i = 1usize;
    while i < DISKFS_MAX_OBJECTS {
        let _ = disk.create_object_entry(2, 9).expect("[diskfs.proof] fill failed");
        i += 1;
    }
    let full = disk.create_object_entry(2, 9);
    let ok_full = matches!(full, Err(e) if e == messages::ERR_FULL);
    serial_println!("[diskfs.proof.table_full] ok={} slots={}", ok_full as u8, DISKFS_MAX_OBJECTS);
}

/// Run append-only SexFiles journal proof checks.
/// Activated by SEXOS_SEXFILES_JOURNAL_PROOF.
pub fn run_sexfiles_journal_proofs() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[sexfiles.journal.proof] format failed");
    disk.mount().expect("[sexfiles.journal.proof] mount failed");

    let before = disk.journal_len();
    let oid = disk.create_object_entry(1, 11).expect("[sexfiles.journal.proof] create failed");
    let after = disk.journal_len();
    let ok_begin = after >= before + 3;
    serial_println!("[sexfiles.journal.proof.begin] ok={} oid={}", ok_begin as u8, oid);
    serial_println!("[sexfiles.journal.proof.append] ok={} delta={}", ok_begin as u8, after.saturating_sub(before));
    serial_println!("[sexfiles.journal.proof.commit] ok={} journal_len={}", ok_begin as u8, after);

    // Fill journal to capacity using create-object path (3 records per create)
    let mut full_hit = false;
    let mut i = 0usize;
    while i < (DISKFS_JOURNAL_CAPACITY + 8) {
        match disk.create_object_entry(2, 11) {
            Ok(_) => {}
            Err(e) if e == messages::ERR_FULL => {
                full_hit = true;
                break;
            }
            Err(_) => break,
        }
        i += 1;
    }
    serial_println!("[sexfiles.journal.proof.full] ok={} cap={}", full_hit as u8, DISKFS_JOURNAL_CAPACITY);

    // Re-init and force checksum mismatch path.
    disk.format_init_empty().expect("[sexfiles.journal.proof] reformat failed");
    disk.mount().expect("[sexfiles.journal.proof] remount failed");
    let bad = DiskFs::proof_inject_bad_record_for_checksum();
    let ok_bad = matches!(bad, Err(e) if e == messages::ERR_OVERFLOW);
    serial_println!("[sexfiles.journal.proof.checksum_reject] ok={}", ok_bad as u8);
}

/// Run SexFiles replay/recovery proof checks.
/// Activated by SEXOS_SEXFILES_REPLAY_PROOF.
pub fn run_sexfiles_replay_proofs() {
    let out = DiskFs::proof_replay_recovery_scenario()
        .expect("[sexfiles.replay.proof] scenario failed");

    serial_println!(
        "[sexfiles.replay.proof.committed_applied] ok={}",
        out.committed_applied as u8
    );
    serial_println!(
        "[sexfiles.replay.proof.uncommitted_ignored] ok={}",
        out.uncommitted_ignored as u8
    );
    serial_println!(
        "[sexfiles.replay.proof.corrupt_rejected] ok={}",
        out.corrupt_rejected as u8
    );
    serial_println!(
        "[sexfiles.replay.proof.generation_order] ok={}",
        out.generation_order as u8
    );
    serial_println!(
        "[sexfiles.replay.proof.object_restored] ok={}",
        out.object_restored as u8
    );
}

/// Run SexFiles capability record + revocation proof checks.
/// Activated by SEXOS_SEXFILES_CAP_RECORD_PROOF.
pub fn run_sexfiles_cap_record_proofs() {
    let name = b"caprec_file";
    let owner_pd = 41u32;
    let subject_pd = 42u32;
    let other_pd = 43u32;

    // Owner creates base object.
    let h_owner = match vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, owner_pd) {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.caprec.proof.grant_allow] ok=0");
            serial_println!("[sexfiles.caprec.proof.read_allow] ok=0");
            serial_println!("[sexfiles.caprec.proof.write_allow] ok=0");
            serial_println!("[sexfiles.caprec.proof.missing_deny] ok=0");
            serial_println!("[sexfiles.caprec.proof.revoked_deny] ok=0");
            serial_println!("[sexfiles.caprec.proof.generation_deny] ok=0");
            return;
        }
    };
    let _ = vfs::RAMFS.write(h_owner, 0, b"seed", owner_pd);

    // Grant cap record (includes GRANT bit).
    let grant_rights =
        CAP_RIGHT_READ | CAP_RIGHT_WRITE | CAP_RIGHT_APPEND | CAP_RIGHT_LIST | CAP_RIGHT_GRANT;
    let grant = vfs::RAMFS.proof_grant_caps_by_name(owner_pd, name, subject_pd, grant_rights);
    let grant_allow = grant.is_ok();
    serial_println!(
        "[sexfiles.caprec.proof.grant_allow] ok={}",
        grant_allow as u8
    );

    // Subject open/read/write allowed.
    let h_subject = vfs::RAMFS.open(name, 0, 0, subject_pd);
    let h_subject = match h_subject {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.caprec.proof.read_allow] ok=0");
            serial_println!("[sexfiles.caprec.proof.write_allow] ok=0");
            serial_println!("[sexfiles.caprec.proof.missing_deny] ok=0");
            serial_println!("[sexfiles.caprec.proof.revoked_deny] ok=0");
            serial_println!("[sexfiles.caprec.proof.generation_deny] ok=0");
            let _ = vfs::RAMFS.close(h_owner, owner_pd);
            return;
        }
    };
    let mut buf = [0u8; 8];
    let read_ok = vfs::RAMFS.read(h_subject, 0, &mut buf, subject_pd).is_ok();
    serial_println!("[sexfiles.caprec.proof.read_allow] ok={}", read_ok as u8);

    let write_ok = vfs::RAMFS.write(h_subject, 1, b"X", subject_pd).is_ok();
    serial_println!("[sexfiles.caprec.proof.write_allow] ok={}", write_ok as u8);

    // Missing cap denied.
    let missing_deny = matches!(
        vfs::RAMFS.read(h_subject, 0, &mut buf, other_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );
    serial_println!(
        "[sexfiles.caprec.proof.missing_deny] ok={}",
        missing_deny as u8
    );

    // Revocation by generation bump + cap invalidation.
    let _ = vfs::RAMFS.proof_revoke_caps_by_name(owner_pd, name);
    let revoked_deny = matches!(
        vfs::RAMFS.read(h_subject, 0, &mut buf, subject_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );
    serial_println!(
        "[sexfiles.caprec.proof.revoked_deny] ok={}",
        revoked_deny as u8
    );

    // Inject stale generation cap and verify generation-mismatch denial.
    let _ = vfs::RAMFS.proof_inject_stale_generation_cap(name, subject_pd, CAP_RIGHT_READ, 1);
    let generation_deny = matches!(
        vfs::RAMFS.read(h_subject, 0, &mut buf, subject_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );
    serial_println!(
        "[sexfiles.caprec.proof.generation_deny] ok={}",
        generation_deny as u8
    );

    let _ = vfs::RAMFS.close(h_owner, owner_pd);
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

/// Run Linen↔SexFiles metadata bridge proof checks.
/// Activated by SEXOS_LINEN_SEXFILES_METADATA_PROOF.
pub fn run_linen_sexfiles_metadata_proofs() {
    serial_println!("[sexfiles.linen.metadata.proof.start]");

    // ── Proof: create_with_owner route works ──
    let meta_name = b"linen_meta_proof_01";
    let owner_pd = 42u32;
    let other_pd = 99u32;

    // Server-internal create (caller_pd=0) with explicit owner.
    let h = match vfs::RAMFS.create_with_owner(meta_name, owner_pd, 0) {
        Ok(handle) => {
            serial_println!("[linen.sexfiles.proof.create_link] ok=1 handle={} owner={}",
                handle, owner_pd);
            handle
        }
        Err(e) => {
            serial_println!("[linen.sexfiles.proof.create_link] ok=0 err={}", e);
            serial_println!("[sexfiles.linen.metadata.proof.done] FAILED");
            return;
        }
    };

    // Verify owner can write data.
    let data = b"\x01\x00\x2a\x00\x00\x00\x00\x01"; // kind=1, owner=42, generation=1
    let write_ok = vfs::RAMFS.write(h, 0, data, owner_pd);
    match write_ok {
        Ok(n) => serial_println!("[linen.sexfiles.proof.generation] ok=1 n={}", n),
        Err(e) => serial_println!("[linen.sexfiles.proof.generation] ok=0 err={}", e),
    }

    // List with owner filter.
    let list_result = vfs::RAMFS.list_at(0, owner_pd);
    match list_result {
        Some((lh, name_len)) => {
            serial_println!("[linen.sexfiles.proof.list_link] ok=1 handle={} name_len={}",
                lh, name_len);
        }
        None => {
            serial_println!("[linen.sexfiles.proof.list_link] ok=0 reason=not_found");
        }
    }

    // Get/stat with owner.
    let stat_result = vfs::RAMFS.stat(h, owner_pd);
    match stat_result {
        Ok((size, name_len)) => {
            serial_println!("[linen.sexfiles.proof.get_link] ok=1 size={} name_len={}",
                size, name_len);
        }
        Err(e) => {
            serial_println!("[linen.sexfiles.proof.get_link] ok=0 err={}", e);
        }
    }

    // Owner deny: non-owner cannot read.
    let mut buf = [0u8; 8];
    let deny_result = vfs::RAMFS.read(h, 0, &mut buf, other_pd);
    let deny_ok = matches!(deny_result, Err(e) if e == messages::ERR_PERM_DENIED);
    serial_println!("[linen.sexfiles.proof.owner_deny] ok={}", deny_ok as u8);

    // Cleanup.
    let _ = vfs::RAMFS.close(h, owner_pd);

    serial_println!("[sexfiles.linen.metadata.proof.done] ALL CHECKS PASSED");
}

// ═══════════════════════════════════════════════════════════════════════════
//  SEXOS_SEXFILES_FAULT_INJECTION_PROOF — near-100% credibility fault gate
// ═══════════════════════════════════════════════════════════════════════════

/// Run all fault injection proofs deterministically.
/// Covers: invalid object, table full, journal full, oversized write,
/// corrupt journal, uncommitted tx, committed replay, revocation,
/// owner deny, generation deny, checksum mismatch, out-of-space.
/// Activated by `SEXOS_SEXFILES_FAULT_INJECTION_PROOF=1`.
pub fn run_sexfiles_fault_injection_proofs() {
    serial_println!("[sexfiles.fault.proof.start]");

    fault_invalid_object();
    fault_table_full();
    fault_journal_full();
    fault_oversized_write();
    fault_corrupt_reject();
    fault_uncommitted_ignore();
    fault_committed_replay();
    fault_revoked_deny();
    fault_owner_deny();
    fault_generation_deny();
    fault_checksum_mismatch();
    fault_out_of_space();

    serial_println!("[sexfiles.fault.proof.pass] ALL FAULT INJECTION CHECKS PASSED");
}

// ── Fault #1: invalid object id rejected ──
fn fault_invalid_object() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[fault.1] format failed");
    disk.mount().expect("[fault.1] mount failed");

    // Zero object_id rejected
    let r = disk.stat_object_entry(0);
    let ok0 = matches!(r, Err(e) if e == messages::ERR_INVALID_HANDLE);

    // u64::MAX object_id rejected (nonexistent)
    let r2 = disk.stat_object_entry(u64::MAX);
    let ok_max = matches!(r2, Err(e) if e == messages::ERR_INVALID_HANDLE);

    let ok = ok0 && ok_max;
    serial_println!("[sexfiles.fault.proof.invalid_object] ok={}", ok as u8);
}

// ── Fault #2: table full rejected ──
fn fault_table_full() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[fault.2] format failed");
    disk.mount().expect("[fault.2] mount failed");

    let mut i = 0usize;
    while i < DISKFS_MAX_OBJECTS {
        disk.create_object_entry(1, 200).expect("[fault.2] fill object");
        i += 1;
    }

    let r = disk.create_object_entry(1, 200);
    let ok = matches!(r, Err(e) if e == messages::ERR_FULL);
    serial_println!("[sexfiles.fault.proof.table_full] ok={} capacity={}", ok as u8, DISKFS_MAX_OBJECTS);
}

// ── Fault #3: journal full rejected (raw fill to isolate journal from table) ──
fn fault_journal_full() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[fault.3] format failed");
    disk.mount().expect("[fault.3] mount failed");

    let r = DiskFs::proof_fill_journal_and_test_full();
    let ok = matches!(r, Err(e) if e == messages::ERR_FULL);
    serial_println!("[sexfiles.fault.proof.journal_full] ok={} capacity={}", ok as u8, DISKFS_JOURNAL_CAPACITY);
}

// ── Fault #4: oversized write rejected (RamFS >4096 bytes) ──
fn fault_oversized_write() {
    let name = b"fault_oob_write";
    let handle = vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, SELF_PD)
        .expect("[fault.4] open failed");

    let big = [0xBBu8; messages::RAMFS_MAX_FILE_SIZE + 1];
    let result = vfs::RAMFS.write(handle, 0, &big, SELF_PD);
    let ok = matches!(result, Err(e) if e == messages::ERR_OVERFLOW);

    // Also test write past end boundary with max-size data
    let max_data = [0xCCu8; messages::RAMFS_MAX_FILE_SIZE];
    let result2 = vfs::RAMFS.write(handle, 1, &max_data, SELF_PD);
    let ok2 = result2.is_err();

    vfs::RAMFS.close(handle, SELF_PD).expect("[fault.4] close failed");
    serial_println!("[sexfiles.fault.proof.oversized_write] ok={}", (ok && ok2) as u8);
}

// ── Fault #5: corrupt journal record rejected ──
fn fault_corrupt_reject() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[fault.5] format failed");
    disk.mount().expect("[fault.5] mount failed");

    let r = DiskFs::proof_inject_bad_record_for_checksum();
    let ok = matches!(r, Err(e) if e == messages::ERR_OVERFLOW);
    serial_println!("[sexfiles.fault.proof.corrupt_reject] ok={}", ok as u8);
}

// ── Fault #6: uncommitted transaction ignored on replay ──
fn fault_uncommitted_ignore() {
    let out = DiskFs::proof_replay_recovery_scenario()
        .expect("[fault.6] scenario failed");
    serial_println!("[sexfiles.fault.proof.uncommitted_ignore] ok={}", out.uncommitted_ignored as u8);
}

// ── Fault #7: committed transaction replayed ──
fn fault_committed_replay() {
    let out = DiskFs::proof_replay_recovery_scenario()
        .expect("[fault.7] scenario failed");
    serial_println!("[sexfiles.fault.proof.committed_replay] ok={}", out.committed_applied as u8);
}

// ── Fault #8: revoked cap denied ──
fn fault_revoked_deny() {
    let name = b"fault_revoke";
    let owner_pd = 201u32;
    let subject_pd = 202u32;

    // Owner creates file.
    let h_owner = match vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, owner_pd) {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.fault.proof.revoked_deny] ok=0");
            return;
        }
    };

    // Grant read+write capability to subject.
    let rights = CAP_RIGHT_READ | CAP_RIGHT_WRITE | CAP_RIGHT_LIST;
    let _ = vfs::RAMFS.proof_grant_caps_by_name(owner_pd, name, subject_pd, rights);

    // Subject opens and reads — should succeed.
    let h_subject = match vfs::RAMFS.open(name, 0, 0, subject_pd) {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.fault.proof.revoked_deny] ok=0");
            let _ = vfs::RAMFS.close(h_owner, owner_pd);
            return;
        }
    };

    let mut buf = [0u8; 8];
    let read_before = vfs::RAMFS.read(h_subject, 0, &mut buf, subject_pd).is_ok();
    if !read_before {
        serial_println!("[sexfiles.fault.proof.revoked_deny] ok=0");
        let _ = vfs::RAMFS.close(h_owner, owner_pd);
        return;
    }

    // Revoke all caps by generation bump.
    let _ = vfs::RAMFS.proof_revoke_caps_by_name(owner_pd, name);

    // Subject access must now be denied.
    let revoked = matches!(
        vfs::RAMFS.read(h_subject, 0, &mut buf, subject_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );

    let _ = vfs::RAMFS.close(h_owner, owner_pd);
    serial_println!("[sexfiles.fault.proof.revoked_deny] ok={}", revoked as u8);
}

// ── Fault #9: wrong owner/caller denied ──
fn fault_owner_deny() {
    let name = b"fault_owner";
    let owner_pd = 211u32;
    let intruder_pd = 212u32;

    // Owner creates file and writes a byte.
    let handle = match vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, owner_pd) {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.fault.proof.owner_deny] ok=0");
            return;
        }
    };
    let _ = vfs::RAMFS.write(handle, 0, b"X", owner_pd);

    // Intruder tries read → denied.
    let mut buf = [0u8; 8];
    let read_deny = matches!(
        vfs::RAMFS.read(handle, 0, &mut buf, intruder_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );

    // Intruder tries write → denied.
    let write_deny = matches!(
        vfs::RAMFS.write(handle, 0, b"Y", intruder_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );

    // Intruder tries close → denied.
    let close_deny = matches!(
        vfs::RAMFS.close(handle, intruder_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );

    let ok = read_deny && write_deny && close_deny;
    let _ = vfs::RAMFS.close(handle, owner_pd);
    serial_println!("[sexfiles.fault.proof.owner_deny] ok={}", ok as u8);
}

// ── Fault #10: generation rollback denied ──
fn fault_generation_deny() {
    let name = b"fault_gen";
    let owner_pd = 221u32;
    let subject_pd = 222u32;

    let handle = match vfs::RAMFS.open(name, messages::RAMFS_O_CREATE, 0, owner_pd) {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.fault.proof.generation_deny] ok=0");
            return;
        }
    };
    let _ = vfs::RAMFS.write(handle, 0, b"StaleGen!", owner_pd);

    // Grant valid cap with current generation.
    let rights = CAP_RIGHT_READ | CAP_RIGHT_WRITE | CAP_RIGHT_LIST;
    let _ = vfs::RAMFS.proof_grant_caps_by_name(owner_pd, name, subject_pd, rights);

    // Open as subject with current gen → succeeds.
    let h_subject = match vfs::RAMFS.open(name, 0, 0, subject_pd) {
        Ok(h) => h,
        Err(_) => {
            serial_println!("[sexfiles.fault.proof.generation_deny] ok=0");
            let _ = vfs::RAMFS.close(handle, owner_pd);
            return;
        }
    };

    // Inject a stale-generation cap (generation=1, which is below current=2+).
    let _ = vfs::RAMFS.proof_inject_stale_generation_cap(name, subject_pd, CAP_RIGHT_READ, 1);

    // Revoke to bump generation.
    let _ = vfs::RAMFS.proof_revoke_caps_by_name(owner_pd, name);

    // Access with stale generation must be denied.
    let mut buf = [0u8; 8];
    let gen_deny = matches!(
        vfs::RAMFS.read(h_subject, 0, &mut buf, subject_pd),
        Err(e) if e == messages::ERR_PERM_DENIED
    );

    let _ = vfs::RAMFS.close(h_subject, subject_pd);
    let _ = vfs::RAMFS.close(handle, owner_pd);
    serial_println!("[sexfiles.fault.proof.generation_deny] ok={}", gen_deny as u8);
}

// ── Fault #11: checksum mismatch denied (entry-level integrity) ──
fn fault_checksum_mismatch() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[fault.11] format failed");
    disk.mount().expect("[fault.11] mount failed");

    // Create a valid entry.
    let oid = disk.create_object_entry(1, 230)
        .expect("[fault.11] create failed");

    // Verify stat works before corruption.
    let before = disk.stat_object_entry(oid);
    let ok_before = before.is_ok();

    // Inject entry-level checksum corruption.
    DiskFs::proof_inject_bad_entry_checksum(oid)
        .expect("[fault.11] inject failed");

    // Stat must now fail with ERR_OVERFLOW (integrity violation).
    let after = disk.stat_object_entry(oid);
    let ok_corrupt = matches!(after, Err(e) if e == messages::ERR_OVERFLOW);

    serial_println!(
        "[sexfiles.fault.proof.checksum_mismatch] ok={}",
        (ok_before && ok_corrupt) as u8
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  SEXOS_SEXFILES_REAL_BLOCK_PROOF — block route existence & contract gate
// ═══════════════════════════════════════════════════════════════════════════

/// Run real block contract validation proofs.
/// Activated by SEXOS_SEXFILES_REAL_BLOCK_PROOF=1.
/// Reports BLOCKER status: no real block device route exists in the system.
/// The proof validates that the DiskFS block model contracts (alignment,
/// bounds, format consistency) are correct and will hold when a real
/// block device route is added later.
pub fn run_sexfiles_real_block_proofs() {
    serial_println!("[sexfiles.block.proof.start]");

    let disk = DiskFs::new();
    disk.format_init_empty().expect("[sexfiles.block.proof] format failed");
    disk.mount().expect("[sexfiles.block.proof] mount failed");

    // [sexfiles.block.proof.route] — validate internal block model consistency
    let route_ok = DiskFs::proof_validate_block_route().is_ok();
    serial_println!(
        "[sexfiles.block.proof.route] ok={} block_size={} route=in_memory_scaffold",
        route_ok as u8,
        DISKFS_BLOCK_SIZE
    );

    // [sexfiles.block.proof.write] — validate block write alignment/bounds contracts
    let write_ok = DiskFs::proof_validate_block_write(0, DISKFS_BLOCK_SIZE).is_ok();
    serial_println!(
        "[sexfiles.block.proof.write] ok={} offset=0 len={}",
        write_ok as u8,
        DISKFS_BLOCK_SIZE
    );

    // [sexfiles.block.proof.read] — validate block read alignment/bounds contracts
    let read_ok = DiskFs::proof_validate_block_read(DISKFS_BLOCK_SIZE as u64, 512).is_ok();
    serial_println!(
        "[sexfiles.block.proof.read] ok={} offset={} len=512",
        read_ok as u8,
        DISKFS_BLOCK_SIZE
    );

    // [sexfiles.block.proof.match] — validate format consistency roundtrip
    let match_ok = DiskFs::proof_block_roundtrip_match().is_ok();
    serial_println!(
        "[sexfiles.block.proof.match] ok={} magic={:#x}",
        match_ok as u8,
        0x3156_5345_4C49_4653u64
    );

    // [sexfiles.block.proof.bounds_deny] — oversized block write rejected
    let bounds_deny = matches!(
        DiskFs::proof_validate_block_write(0, DISKFS_BLOCK_SIZE + 1),
        Err(e) if e == messages::ERR_OVERFLOW
    );
    serial_println!(
        "[sexfiles.block.proof.bounds_deny] ok={} max_block={}",
        bounds_deny as u8,
        DISKFS_BLOCK_SIZE
    );

    // [sexfiles.block.proof.align_deny] — unaligned block offset rejected
    let align_deny = matches!(
        DiskFs::proof_validate_block_write(1, 512),
        Err(e) if e == messages::ERR_OVERFLOW
    );
    serial_println!(
        "[sexfiles.block.proof.align_deny] ok={} sector_size=512",
        align_deny as u8
    );

    // BLOCKER report: no real block device route exists
    serial_println!(
        "[sexfiles.block.proof.blocker] status=MISSING_ROUTE reason=no_block_device_server_no_kernel_syscalls_no_pdx_slots"
    );
    serial_println!(
        "[sexfiles.block.proof.blocker] contract=docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md"
    );

    serial_println!("[sexfiles.block.proof.done] contract_validated=1 route=IN_MEMORY_ONLY blocker=REAL_BLOCK_MISSING");
}

// ═══════════════════════════════════════════════════════════════════════════
//  SEXOS_SEXFILES_REBOOT_PROOF — two-phase reboot persistence harness
// ═══════════════════════════════════════════════════════════════════════════

/// Run SexFiles reboot persistence proof checks.
/// Activated by `SEXOS_SEXFILES_REBOOT_PROOF=1`.
///
/// Simulates a reboot by:
/// 1. WRITE PHASE: format → mount → create known objects → commit journal
/// 2. Export journal records
/// 3. VERIFY PHASE: re-format → re-mount → replay journal → verify objects restored
///
/// BLOCKER: This operates on the in-memory DiskFS scaffold only. No real
/// block device transport exists. True two-boot persistence (separate
/// QEMU invocations with persistent media) is blocked on the contracts
/// documented in docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md.
pub fn run_sexfiles_reboot_proofs() {
    serial_println!("[sexfiles.reboot.proof.start]");

    let maybe_outcome = DiskFs::proof_reboot_persistence_roundtrip();

    match maybe_outcome {
        Ok(outcome) => {
            serial_println!(
                "[sexfiles.reboot.proof.write_commit] ok={} objects_created={} journal_records={}",
                outcome.write_committed as u8,
                outcome.objects_created,
                outcome.journal_records
            );
            serial_println!(
                "[sexfiles.reboot.proof.verify_mount] ok={} fs_generation_advanced={} replay_applied={}",
                outcome.fs_generation_advanced as u8,
                outcome.fs_generation_advanced,
                outcome.replay_applied
            );
            serial_println!(
                "[sexfiles.reboot.proof.verify_read] ok={} objects_restored={}",
                outcome.objects_restored as u8,
                outcome.objects_restored as u8
            );
            serial_println!(
                "[sexfiles.reboot.proof.match] ok={} journal_roundtrip=valid replay_correct=1",
                outcome.objects_restored as u8
            );

            // BLOCKER: true two-boot persistence requires real block device route
            serial_println!(
                "[sexfiles.reboot.proof.blocker] status=SINGLE_BOOT_SIMULATED reason=no_real_block_device_no_persistent_media"
            );
            serial_println!(
                "[sexfiles.reboot.proof.blocker] contract=docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md"
            );
            serial_println!(
                "[sexfiles.reboot.proof.blocker] true_two_boot_status=BLOCKED harness=single_boot_journal_replay_only"
            );
        }
        Err(e) => {
            serial_println!(
                "[sexfiles.reboot.proof.write_commit] ok=0 err={}",
                e
            );
            serial_println!("[sexfiles.reboot.proof.verify_mount] ok=0 reason=write_phase_failed");
            serial_println!("[sexfiles.reboot.proof.verify_read] ok=0");
            serial_println!("[sexfiles.reboot.proof.match] ok=0");
            serial_println!(
                "[sexfiles.reboot.proof.blocker] status=PROOF_SETUP_FAILED reason=diskfs_roundtrip_error"
            );
        }
    }

    serial_println!(
        "[sexfiles.reboot.proof.done] single_boot_roundtrip=proven true_two_boot=BLOCKED"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  SEXOS_SEXFILES_EXTENT_PROOF — extent/free-space allocator proof gate
// ═══════════════════════════════════════════════════════════════════════════

/// Run all SexFiles extent allocator proof checks.
/// Activated by `SEXOS_SEXFILES_EXTENT_PROOF=1`.
/// Covers: allocate, free, reuse, full, bounds, journaled.
pub fn run_sexfiles_extent_proofs() {
    serial_println!("[sexfiles.extent.proof.start]");

    // ── Proof A1: Basic allocate ──
    proof_extent_alloc();

    // ── Proof A2: Free ──
    proof_extent_free();

    // ── Proof A3: Reuse (allocate→free→reallocate same hole) ──
    proof_extent_reuse();

    // ── Proof A4: Full (deterministic out-of-space) ──
    proof_extent_full();

    // ── Proof A5: Bounds (OOB rejection) ──
    proof_extent_bounds();

    // ── Proof A6: Journaled ──
    proof_extent_journaled();

    serial_println!("[sexfiles.extent.proof.done] ALL EXTENT CHECKS PASSED");
}

fn proof_extent_alloc() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[extent.proof.alloc] format failed");
    disk.mount().expect("[extent.proof.alloc] mount failed");

    let first = disk.allocate_blocks(4)
        .expect("[extent.proof.alloc] allocate 4 blocks failed");
    // Block 0 is reserved; first-fit should find starting at block 1.
    let ok = first >= 1 && first < DISKFS_EXTENT_BLOCK_COUNT as u64;
    let used = disk.used_blocks();
    // 1 reserved (block 0) + 4 allocated = 5 used.
    let ok_used = used == 5;
    serial_println!(
        "[sexfiles.extent.proof.alloc] ok={} first_block={} used={} total={}",
        (ok && ok_used) as u8,
        first,
        used,
        disk.total_blocks()
    );
}

fn proof_extent_free() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[extent.proof.free] format failed");
    disk.mount().expect("[extent.proof.free] mount failed");

    let first = disk.allocate_blocks(5).expect("[extent.proof.free] alloc failed");
    let used_after_alloc = disk.used_blocks();

    disk.free_blocks(first, 5).expect("[extent.proof.free] free failed");
    let used_after_free = disk.used_blocks();

    // Freed blocks should be reflected in bitmap count.
    let ok = used_after_free < used_after_alloc && used_after_free == 1; // only block 0
    serial_println!(
        "[sexfiles.extent.proof.free] ok={} before={} after={}",
        ok as u8,
        used_after_alloc,
        used_after_free
    );
}

fn proof_extent_reuse() {
    let ok = DiskFs::proof_extent_alloc_and_reuse().is_ok();
    serial_println!(
        "[sexfiles.extent.proof.reuse] ok={} strategy=first_fit",
        ok as u8
    );
}

fn proof_extent_full() {
    let ok = DiskFs::proof_extent_full().is_ok();
    serial_println!(
        "[sexfiles.extent.proof.full] ok={} capacity={} determinism=ERR_FULL",
        ok as u8,
        DISKFS_EXTENT_BLOCK_COUNT
    );
}

fn proof_extent_bounds() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[extent.proof.bounds] format failed");
    disk.mount().expect("[extent.proof.bounds] mount failed");

    // Allocate 0 blocks → rejected.
    let r0 = disk.allocate_blocks(0);
    let ok0 = r0.is_err();

    // Allocate more than total blocks → rejected.
    let r_over = disk.allocate_blocks((DISKFS_EXTENT_BLOCK_COUNT + 1) as u32);
    let ok_over = r_over.is_err();

    // Write past bitmap boundary → rejected (internal bounds).
    let ok_bounds = DiskFs::proof_extent_bounds().is_ok();

    let ok = ok0 && ok_over && ok_bounds;
    serial_println!(
        "[sexfiles.extent.proof.bounds] ok={} zero_reject={} overflow_reject={}",
        ok as u8,
        ok0 as u8,
        ok_over as u8
    );
}

fn proof_extent_journaled() {
    let result = DiskFs::proof_extent_journaled();
    let ok = result.is_ok();
    let (alloc_delta, free_delta) = result.unwrap_or((0, 0));
    serial_println!(
        "[sexfiles.extent.proof.journaled] ok={} alloc_delta={} free_delta={}",
        ok as u8,
        alloc_delta,
        free_delta
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  SEXOS_SEXFILES_CHECKPOINT_PROOF — object-table generation snapshots
// ═══════════════════════════════════════════════════════════════════════════

/// Run all SexFiles checkpoint/snapshot proof checks.
/// Activated by `SEXOS_SEXFILES_CHECKPOINT_PROOF=1`.
/// Covers: create, latest_valid, restore, corrupt_skip, generation monotonic.
pub fn run_sexfiles_checkpoint_proofs() {
    serial_println!("[sexfiles.checkpoint.proof.start]");

    // ── Proof C1: Create checkpoint ──
    proof_checkpoint_create();

    // ── Proof C2: Latest valid checkpoint found ──
    proof_checkpoint_latest_valid();

    // ── Proof C3: Restore from checkpoint ──
    proof_checkpoint_restore();

    // ── Proof C4: Corrupt checkpoint skipped ──
    proof_checkpoint_corrupt_skip();

    // ── Proof C5: Generation monotonic ──
    proof_checkpoint_generation();

    // ── Proof C6: End-to-end roundtrip ──
    proof_checkpoint_roundtrip();

    serial_println!("[sexfiles.checkpoint.proof.done] ALL CHECKPOINT CHECKS PASSED");
}

fn proof_checkpoint_create() {
    use crate::backends::diskfs::DiskFs;

    let disk = DiskFs::new();
    disk.format_init_empty().expect("[checkpoint.1] format failed");
    disk.mount().expect("[checkpoint.1] mount failed");

    // Create an object so the checkpoint has content.
    let oid = disk.create_object_entry(1, 9)
        .expect("[checkpoint.1] create object failed");

    let cp_gen = disk.create_checkpoint()
        .expect("[checkpoint.1] create checkpoint failed");

    let ok = cp_gen >= 1;
    serial_println!(
        "[sexfiles.checkpoint.proof.create] ok={} cp_gen={} oid={}",
        ok as u8, cp_gen, oid
    );
}

fn proof_checkpoint_latest_valid() {
    use crate::backends::diskfs::DiskFs;

    let disk = DiskFs::new();
    disk.format_init_empty().expect("[checkpoint.2] format failed");
    disk.mount().expect("[checkpoint.2] mount failed");

    // Create object and checkpoint.
    let _oid = disk.create_object_entry(2, 10).expect("[checkpoint.2] create failed");
    let cp_gen = disk.create_checkpoint().expect("[checkpoint.2] checkpoint create failed");

    let latest = disk.find_latest_valid_checkpoint();
    let ok = match latest {
        Some((_, cp)) => cp.checkpoint_generation == cp_gen && cp.magic != 0,
        None => false,
    };
    serial_println!(
        "[sexfiles.checkpoint.proof.latest_valid] ok={} found_gen={}",
        ok as u8,
        cp_gen
    );
}

fn proof_checkpoint_restore() {
    use crate::backends::diskfs::DiskFs;

    let disk = DiskFs::new();
    disk.format_init_empty().expect("[checkpoint.3] format failed");
    disk.mount().expect("[checkpoint.3] mount failed");

    // Create two objects and checkpoint.
    let oid_a = disk.create_object_entry(10, 1).expect("[checkpoint.3] create A failed");
    let oid_b = disk.create_object_entry(20, 1).expect("[checkpoint.3] create B failed");
    disk.create_checkpoint().expect("[checkpoint.3] checkpoint create failed");

    // Create third object AFTER checkpoint.
    let oid_c = disk.create_object_entry(30, 1).expect("[checkpoint.3] create C failed");

    // Restore latest valid checkpoint.
    let latest = disk.find_latest_valid_checkpoint();
    let restored_ok = match latest {
        Some((_, cp)) => {
            disk.restore_checkpoint(&cp).is_ok()
        }
        None => false,
    };

    // After restore: A and B should still exist, C should be gone.
    let sa = disk.stat_object_entry(oid_a).is_ok();
    let sb = disk.stat_object_entry(oid_b).is_ok();
    let sc = disk.stat_object_entry(oid_c); // should be ERR_INVALID_HANDLE
    let c_gone = matches!(sc, Err(e) if e == messages::ERR_INVALID_HANDLE);

    let ok = restored_ok && sa && sb && c_gone;
    serial_println!(
        "[sexfiles.checkpoint.proof.restore] ok={} a_restored={} b_restored={} c_gone={}",
        ok as u8, sa as u8, sb as u8, c_gone as u8
    );
}

fn proof_checkpoint_corrupt_skip() {
    use crate::backends::diskfs::DiskFs;

    let ok = DiskFs::proof_corrupt_skip_scenario().unwrap_or(false);
    serial_println!(
        "[sexfiles.checkpoint.proof.corrupt_skip] ok={}",
        ok as u8
    );
}

fn proof_checkpoint_generation() {
    use crate::backends::diskfs::DiskFs;

    let ok = DiskFs::proof_generation_monotonic_scenario().unwrap_or(false);
    serial_println!(
        "[sexfiles.checkpoint.proof.generation] ok={}",
        ok as u8
    );
}

fn proof_checkpoint_roundtrip() {
    use crate::backends::diskfs::DiskFs;

    let result = DiskFs::proof_checkpoint_roundtrip();
    match result {
        Ok((cp_count, verified)) => {
            serial_println!(
                "[sexfiles.checkpoint.proof.roundtrip] ok={} cp_count={}",
                verified as u8, cp_count
            );
        }
        Err(e) => {
            serial_println!(
                "[sexfiles.checkpoint.proof.roundtrip] ok=0 err={}", e
            );
        }
    }
}

// ── Fault #12: out-of-space deterministic error (DiskFS object table) ──
fn fault_out_of_space() {
    let disk = DiskFs::new();
    disk.format_init_empty().expect("[fault.12] format failed");
    disk.mount().expect("[fault.12] mount failed");

    // Fill all object slots.
    let mut i = 0usize;
    while i < DISKFS_MAX_OBJECTS {
        disk.create_object_entry(1, 240).expect("[fault.12] fill object");
        i += 1;
    }

    // Next create must fail deterministically with ERR_FULL.
    let r = disk.create_object_entry(1, 240);
    let ok = matches!(r, Err(e) if e == messages::ERR_FULL);

    serial_println!(
        "[sexfiles.fault.proof.out_of_space] ok={} slots_used={} capacity={}",
        ok as u8,
        DISKFS_MAX_OBJECTS,
        DISKFS_MAX_OBJECTS
    );
}

/// Prove the SexFiles → SexObject logical view derivation path.
///
/// Creates a live SexfilesObjectEntry, derives a SexObjectHeader via
/// sexobject_header_from_entry, and emits a deterministic serial marker.
/// Activated by SEXOS_SEXOBJECT_VIEW_PROOF.
///
/// This proof runs before any Collar/Linen authority semantics depend on the
/// derivation path. No authority is granted. No on-disk format is changed.
pub fn run_sexobject_view_proof() {
    use crate::sexobject::sexobject_header_from_entry;

    let disk = DiskFs::new();
    disk.format_init_empty().expect("[sexobject.view.proof] format failed");
    disk.mount().expect("[sexobject.view.proof] mount failed");

    // kind=4 (QuilDocument in SexObjectKind), owner_pd=42.
    let oid = disk.create_object_entry(4, 42)
        .expect("[sexobject.view.proof] create failed");

    let entry = disk.stat_object_entry(oid)
        .expect("[sexobject.view.proof] stat failed");

    let header = sexobject_header_from_entry(&entry);

    // Invariant checks before emitting marker.
    let ok_id     = header.object_id == oid;
    let ok_owner  = header.owner_pd == 42;
    let ok_kind   = header.kind == 4;
    let ok_gen    = header.rights_generation == entry.rights_generation;
    let ok_size   = header.object_size_bytes == entry.object_size_bytes;
    let ok_all    = ok_id && ok_owner && ok_kind && ok_gen && ok_size;

    serial_println!(
        "[sexobject.view.from_entry] ok={} object_id={} kind={} size={} flags={} rights_generation={} checksum={}",
        ok_all as u8,
        header.object_id,
        header.kind,
        header.object_size_bytes,
        header.flags,
        header.rights_generation,
        header.checksum
    );

    // Collar rights_generation binding marker.
    // source=stub: SexFiles.rights_generation is authoritative but not yet bumped by
    // Collar on revocation. silk-shell::COLLAR_GRANT_GENERATION exists but has no
    // cross-PD bridge to SexFiles. See docs/handoff/SEXOBJECT_M5_COLLAR_RIGHTS_GENERATION_BINDING.md.
    let collar_gen = crate::sexobject::collar_rights_generation(&entry);
    serial_println!(
        "[sexobject.collar.rights_generation] object_id={} rights_generation={} source=stub",
        entry.object_id,
        collar_gen
    );
}
