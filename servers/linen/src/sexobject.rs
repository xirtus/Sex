// Pure logical view adapter: LinenObject → SexObjectHeader / SexObjectKind.
//
// No mutation. No allocation. No I/O. No PDX op. No behavior change.
// Lives here, not in sex-object-model, so the model crate stays dependency-free.
//
// V1 GAPS (documented, not bugs):
//   object_id      — Linen-local monotonic ID; NOT the same space as SexFiles NEXT_OBJECT_ID.
//                    These two namespaces unify in a future migration step.
//   first_block    — LinenObject.ramfs_handle is a RamFS file handle, not a block ref.
//                    Set to 0 until Linen objects bind to SexFiles block addresses.
//   rights_generation — 0 until Collar is bound (M3).
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

#[allow(dead_code)]
/// Construct a logical SexObjectHeader view from a LinenObject.
///
/// V1 gaps: object_id is Linen-local, first_block = 0, rights_generation = 0,
/// checksum = 0. See module doc for full gap table.
pub fn sexobject_header_from_linen(obj: &LinenObject) -> SexObjectHeader {
    SexObjectHeader {
        object_id:           obj.object_id,           // Linen-local; not SexFiles object_id
        content_generation:  obj.generation,           // LinenObject tracks this already
        rights_generation:   0,                        // V1: Collar not bound yet (M3)
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

#[allow(dead_code)]
/// Build a SexObjectRef from a LinenObject using its local generation.
/// Note: object_id is Linen-local — not interchangeable with SexFiles refs.
pub fn linen_object_ref(obj: &LinenObject) -> SexObjectRef {
    SexObjectRef::new(obj.object_id, obj.generation)
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
