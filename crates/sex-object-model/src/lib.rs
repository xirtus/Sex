#![no_std]

// SexObject is the planned canonical durable typed semantic unit of SexOS.
// This crate defines the model types only — no storage ops, no PDX opcodes,
// no kernel edits, no Collar/SexFiles/sexshop behavior changes.
//
// Naming canon (from docs/handoff/SEXOBJECT_CANONICAL_MODEL_V4.md):
//   SexObject     = the concept; never a struct
//   SexObjectHeader = fixed 80-byte logical header (this file)
//   SexObjectKind   = bounded kind enum; discriminants fit SexfilesObjectEntry.kind:u16
//   SexObjectRef    = opaque cross-PD ref; fits PdxMessage.arg0 + arg1
//   SexObjectRecord = preferred future name for extended SexFiles storage entry

// ── Flags (SexObjectHeader.flags bits) ──────────────────────────────────────

pub const FLAG_TOMBSTONED: u32 = 0x01; // deleted; deny writes; generation readable
pub const FLAG_SEALED:     u32 = 0x02; // immutable; deny further content writes
pub const FLAG_REDACTED:   u32 = 0x04; // policy filtering active on stat
pub const FLAG_MIGRATING:  u32 = 0x08; // moving between backends; deny reads

// ── SexObjectKind ────────────────────────────────────────────────────────────

/// Bounded semantic kind for a SexObject.
///
/// repr(u32) in model; all V1 discriminants fit u16 (≤ 65535) to match
/// SexfilesObjectEntry.kind:u16. Callers cast as_u16() on storage write,
/// widen from_u16() on storage read.
///
/// Discriminant 7 is reserved (SceneSnapshot deferred — raw ptr fields).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SexObjectKind {
    RawBlob        = 0,
    AppManifest    = 1,
    AppState       = 2,
    LinenProject   = 3,
    QuilDocument   = 4,
    SpindleSession = 5,
    BellEvent      = 6,
    // 7 reserved — SceneSnapshot deferred (has raw ptr fields; not durable)
    CollarGrant    = 8,
    MeshFact       = 9,
    CrashReport    = 10,
    Package        = 11,
}

impl SexObjectKind {
    /// Convert from u16 storage discriminant. Returns None for unknown or
    /// reserved values (including 7).
    pub const fn from_u16(v: u16) -> Option<Self> {
        match v {
            0  => Some(Self::RawBlob),
            1  => Some(Self::AppManifest),
            2  => Some(Self::AppState),
            3  => Some(Self::LinenProject),
            4  => Some(Self::QuilDocument),
            5  => Some(Self::SpindleSession),
            6  => Some(Self::BellEvent),
            8  => Some(Self::CollarGrant),
            9  => Some(Self::MeshFact),
            10 => Some(Self::CrashReport),
            11 => Some(Self::Package),
            _  => None,
        }
    }

    /// Lossless cast to u16 for SexfilesObjectEntry.kind storage.
    /// All V1 discriminants fit u16.
    pub const fn as_u16(self) -> u16 {
        self as u32 as u16
    }
}

// ── SexObjectRef ─────────────────────────────────────────────────────────────

/// Opaque cross-PD object reference. Zero ABI change — fits PdxMessage.arg0 + arg1.
///
/// object_id  → arg0  (SexFiles NEXT_OBJECT_ID; 0 = null sentinel)
/// generation → arg1  (use rights_generation for capability checking;
///                     use content_generation for content version)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SexObjectRef {
    pub object_id:  u64,
    pub generation: u64,
}

impl SexObjectRef {
    pub const NULL: Self = Self { object_id: 0, generation: 0 };

    pub const fn new(object_id: u64, generation: u64) -> Self {
        Self { object_id, generation }
    }

    /// True if object_id is 0 (invalid sentinel).
    pub const fn is_null(self) -> bool {
        self.object_id == 0
    }
}

// ── SexObjectHeader ──────────────────────────────────────────────────────────

/// Fixed-size logical header for a SexObject.
///
/// Layout (repr(C), 80 bytes):
///   offset  0: object_id          u64   (8)
///   offset  8: content_generation u64   (8)
///   offset 16: rights_generation  u64   (8)
///   offset 24: metadata_generation u64  (8)
///   offset 32: object_size_bytes  u64   (8)
///   offset 40: first_block        u64   (8)
///   offset 48: owner_pd           u32   (4)
///   offset 52: kind               u32   (4)  — SexObjectKind discriminant
///   offset 56: checksum           u32   (4)
///   offset 60: flags              u32   (4)  — FLAG_* bits
///   offset 64: reserved0          u64   (8)  — zero in V1
///   offset 72: reserved1          u64   (8)  — zero in V1
///   total: 80 bytes
///
/// Generation semantics (three distinct counters):
///   content_generation   — content/payload write epoch (new field; proxy = metadata_generation in V1)
///   rights_generation    — capability/revocation epoch (SexFiles authoritative)
///   metadata_generation  — metadata-write epoch
///
/// V1: constructed as a logical view from SexfilesObjectEntry + defaults.
/// No disk format change until M2.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SexObjectHeader {
    pub object_id:           u64,
    pub content_generation:  u64,
    pub rights_generation:   u64,
    pub metadata_generation: u64,
    pub object_size_bytes:   u64,
    pub first_block:         u64,
    pub owner_pd:            u32,
    pub kind:                u32,
    pub checksum:            u32,
    pub flags:               u32,
    pub reserved0:           u64,
    pub reserved1:           u64,
}

impl SexObjectHeader {
    /// Construct a capability SexObjectRef using rights_generation.
    /// Use this ref when passing to PDX for Collar-checked access.
    pub const fn cap_ref(&self) -> SexObjectRef {
        SexObjectRef::new(self.object_id, self.rights_generation)
    }

    /// Construct a content SexObjectRef using content_generation.
    /// Use this ref when identifying a specific content version.
    pub const fn object_ref(&self) -> SexObjectRef {
        SexObjectRef::new(self.object_id, self.content_generation)
    }

    /// True if tombstoned.
    pub const fn is_tombstoned(&self) -> bool {
        self.flags & FLAG_TOMBSTONED != 0
    }

    /// True if sealed (immutable).
    pub const fn is_sealed(&self) -> bool {
        self.flags & FLAG_SEALED != 0
    }

    /// True if redacted (policy filtering active).
    pub const fn is_redacted(&self) -> bool {
        self.flags & FLAG_REDACTED != 0
    }

    /// Decode kind. Returns None if stored discriminant is unknown or reserved.
    pub const fn kind(&self) -> Option<SexObjectKind> {
        if self.kind > u16::MAX as u32 {
            return None;
        }
        SexObjectKind::from_u16(self.kind as u16)
    }
}

// ── Compile-time size assertions ─────────────────────────────────────────────

const _: () = assert!(
    core::mem::size_of::<SexObjectHeader>() == 80,
    "SexObjectHeader must be exactly 80 bytes"
);

const _: () = assert!(
    core::mem::size_of::<SexObjectRef>() == 16,
    "SexObjectRef must be exactly 16 bytes"
);
