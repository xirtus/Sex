// Pure logical view adapter: LinenObject → SexObjectHeader / SexObjectKind.
//
// No mutation. No allocation. No I/O. No PDX op. No behavior change.
// Lives here, not in sex-object-model, so the model crate stays dependency-free.
//
// OQ5 RESOLVED: object_id field now uses sexfiles_object_id (global) when bound,
// not the Linen-local monotonic id. Linen-local id remains for session/UI indexing
// but is NEVER authority-bearing in a SexObjectRef.
//
// REMAINING GAPS (documented, not bugs):
//   first_block    — LinenObject.ramfs_handle is a RamFS file handle, not a block ref.
//                    Set to 0 until Linen objects bind to SexFiles block addresses.
//   rights_generation — 0 until Collar is bound (M6).
//   checksum       — 0; Linen does not compute XOR checksums (SexFiles does).
//   flags          — 0; LinenObject.flags:u8 bit semantics differ from SexObjectHeader.flags:u32.
//
// Kind mapping is approximate (3 Linen variants → 12 SexObjectKind variants):
//   ObjectKind::Document → SexObjectKind::QuilDocument
//   ObjectKind::Session  → SexObjectKind::SpindleSession
//   ObjectKind::Unknown  → SexObjectKind::RawBlob

use crate::session::{LinenObject, ObjectKind};
use sex_object_model::{SexObjectHeader, SexObjectKind, SexObjectRef};

/// Map a Linen ObjectKind to the nearest SexObjectKind.
/// Approximate — 3 Linen variants collapse into 3 of 12 SexObjectKind values.
#[allow(dead_code)]
pub const fn linen_kind_to_sex(kind: ObjectKind) -> SexObjectKind {
    match kind {
        ObjectKind::Document => SexObjectKind::QuilDocument,
        ObjectKind::Session  => SexObjectKind::SpindleSession,
        ObjectKind::Unknown  => SexObjectKind::RawBlob,
    }
}

/// Construct a logical SexObjectHeader view from a LinenObject.
///
/// OQ5 RESOLVED: object_id uses sexfiles_object_id (global, authoritative) when
/// bound (≥1). If not yet bound, object_id=0 (invalid/unbound indicator).
/// Linen-local object_id is NEVER used as the authority-bearing object_id.
///
/// Remaining gaps: first_block = 0, rights_generation = 0, checksum = 0.
/// See module doc for full gap table.
#[allow(dead_code)]
pub fn sexobject_header_from_linen(obj: &LinenObject) -> SexObjectHeader {
    SexObjectHeader {
        // [sexobject.oq5.ref_global] — use global SexFiles id, not Linen-local
        object_id:           obj.sexfiles_object_id,  // OQ5: global SexFiles ID (0 if unbound)
        content_generation:  obj.generation,           // LinenObject tracks this already
        rights_generation:   0,                        // V1: Collar not bound yet (M6)
        metadata_generation: obj.generation,
        object_size_bytes:   0,                        // V1: Linen doesn't track content size
        first_block:         0,                        // V1: ramfs_handle ≠ block ref
        owner_pd:            obj.owner_pd,
        kind:                linen_kind_to_sex(obj.kind) as u32,
        checksum:            0,                        // V1: Linen doesn't compute checksums
        flags:               0,                        // V1: LinenObject.flags:u8 not mapped yet
        reserved0:           0,
        reserved1:           0,
    }
}

/// Build a SexObjectRef from a LinenObject.
///
/// OQ5 RESOLVED: object_id field uses sexfiles_object_id (global, authoritative).
/// Linen-local object_id is NEVER used as the authority-bearing id in a SexObjectRef.
/// If sexfiles_object_id is 0 (not yet bound), the ref carries 0 as a sentinel.
///
/// [sexobject.oq5.local_id_reject] — local id excluded from authority ref
/// [sexobject.oq5.ref_global] — global SexFiles id used in ref
#[allow(dead_code)]
pub fn linen_object_ref(obj: &LinenObject) -> SexObjectRef {
    SexObjectRef::new(obj.sexfiles_object_id, obj.generation)
}

// Compile-time size assertions from linen build context.
const _: () = assert!(
    core::mem::size_of::<SexObjectHeader>() == 80,
    "SexObjectHeader must be 80 bytes"
);
const _: () = assert!(
    core::mem::size_of::<SexObjectRef>() == 16,
    "SexObjectRef must be 16 bytes"
);
