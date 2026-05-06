// Pure logical view adapter: SexfilesObjectEntry → SexObjectHeader.
//
// No disk writes. No PDX ops. No behavior change. No allocation.
// Lives here, not in sex-object-model, so the model crate stays dependency-free.
//
// Field mapping (V1 proxies noted):
//   content_generation  ← metadata_generation  (V1 proxy; real field added in M2 disk extension)
//   flags               ← 0 if in_use, FLAG_TOMBSTONED if not
//
// See docs/handoff/SEXOBJECT_SEXFILES_VIEW_M2.md for full mapping table.

use crate::backends::diskfs::SexfilesObjectEntry;
use sex_object_model::{FLAG_TOMBSTONED, SexObjectHeader};

// Re-export for callers that want to avoid a direct sex-object-model import.
pub use sex_object_model::SexObjectRef;

/// Construct a logical SexObjectHeader view from a SexfilesObjectEntry.
///
/// Pure, const-friendly, no allocation, no mutation, no I/O.
/// `content_generation` is a V1 proxy using `metadata_generation` until
/// `SexObjectRecord` gains a dedicated field in M2 disk extension.
pub fn sexobject_header_from_entry(entry: &SexfilesObjectEntry) -> SexObjectHeader {
    SexObjectHeader {
        object_id:           entry.object_id,
        content_generation:  entry.metadata_generation, // V1 proxy
        rights_generation:   entry.rights_generation,
        metadata_generation: entry.metadata_generation,
        object_size_bytes:   entry.object_size_bytes,
        first_block:         entry.first_block,
        owner_pd:            entry.owner_pd,
        kind:                entry.kind as u32,
        checksum:            entry.checksum,
        flags:               if entry.in_use { 0 } else { FLAG_TOMBSTONED },
        reserved0:           0,
        reserved1:           0,
    }
}

/// Return the rights_generation for a SexObject view derived from a SexFiles entry.
///
/// SOURCE: stub (source=sexfiles_local).
///
/// SexFiles initialises rights_generation=1 on create and is the authoritative store
/// for this value (OQ4, V4 canon doc). However, Collar does NOT yet send a PDX op to
/// SexFiles to bump rights_generation when a CollarGrant is revoked. Until that
/// revocation bridge exists, this value never increases beyond its initial 1.
///
/// Real source (not yet wired):
///   silk-shell::COLLAR_GRANT_GENERATION (static mut u64, starts at 1, wrapping_add per grant)
///   silk-shell::CollarGrant.generation  (per-grant epoch)
///   Neither is accessible cross-PD without a new PDX opcode.
///
/// When the Collar→SexFiles revocation PDX op is wired (future M5 Collar binding),
/// this function is the single call site to replace with the real source. The marker
/// source field changes from "stub" to "real" at that point.
#[allow(dead_code)]
pub fn collar_rights_generation(entry: &SexfilesObjectEntry) -> u64 {
    entry.rights_generation
}

// Compile-time size verification from sexfiles side.
const _: () = assert!(
    core::mem::size_of::<SexObjectHeader>() == 80,
    "SexObjectHeader must be 80 bytes"
);
const _: () = assert!(
    core::mem::size_of::<SexObjectRef>() == 16,
    "SexObjectRef must be 16 bytes"
);
