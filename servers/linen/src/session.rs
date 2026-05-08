//! Linen session/object model.
//!
//! Bounded, no_std, no-heap session layer for Linen objects.
//! Each object has an owner PD, bounded display name, and optional
//! link to a SexFiles/RamFS handle. No POSIX paths, no filesystem semantics.
//!
//! CONTRACT:
//! - Max 16 objects per session.
//! - Display names ≤ 24 bytes.
//! - Object kind is one of: Document, Session, Unknown.
//! - Owner PD validated on all operations.
//! - Non-owner access returns ERR_PERM_DENIED.

use crate::{LINEN_MAX_NAME, LINEN_MAX_OBJECTS};

/// Linen object kind. Matches existing `LinenObjectKind` pattern in silk-shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectKind {
    Document = 0,
    Session = 1,
    Unknown = 2,
}

/// A single Linen session object. All fields bounded.
#[derive(Debug, Clone, Copy)]
pub struct LinenObject {
    /// Unique object ID (auto-incrementing, ≥1).
    pub object_id: u64,
    /// Kind discriminator.
    pub kind: ObjectKind,
    /// Creator/owner PD. Only this PD may access the object.
    pub owner_pd: u32,
    /// Display name (bounded to LINEN_MAX_NAME bytes).
    pub name: [u8; LINEN_MAX_NAME],
    /// Actual length of display name in bytes.
    pub name_len: u8,
    /// Optional SexFiles/RamFS handle (0 = unlinked).
    /// Set when a RamFS file is created for this object's content.
    pub ramfs_handle: u64,
    /// Global SexFiles-assigned object_id (0 = not yet bound).
    /// Obtained via OP_RAMFS_OBJECT_ID after create. Authoritative for
    /// SexObjectRef — NOT the same as the Linen-local object_id above.
    pub sexfiles_object_id: u64,
    /// Metadata generation. Incremented on each metadata update.
    /// Synced with SexFiles object record generation.
    pub generation: u64,
    /// Flags byte. Bit 0 = persisted (1 if backed by SexFiles).
    pub flags: u8,
}

/// Session manager: fixed-size, no-heap object table.
pub struct Session {
    /// Fixed-size object slots.
    objects: [Option<LinenObject>; LINEN_MAX_OBJECTS],
    /// Monotonic ID counter (0 reserved as invalid).
    next_id: u64,
}

impl Session {
    /// Create a new empty session.
    pub const fn new() -> Self {
        const NONE: Option<LinenObject> = None;
        Self {
            objects: [NONE; LINEN_MAX_OBJECTS],
            next_id: 1,
        }
    }

    /// Create a new Linen object.
    ///
    /// Returns the new object_id on success, or:
    /// - `-1` if table is full
    /// - `-2` if name is empty or too long
    pub fn create(&mut self, kind: ObjectKind, name: &[u8], owner_pd: u32) -> Result<u64, i64> {
        // Validate name bounds.
        if name.is_empty() || name.len() > LINEN_MAX_NAME {
            return Err(-2);
        }

        // Find a free slot.
        let mut slot_idx = LINEN_MAX_OBJECTS;
        for (i, slot) in self.objects.iter().enumerate() {
            if slot.is_none() {
                slot_idx = i;
                break;
            }
        }
        if slot_idx == LINEN_MAX_OBJECTS {
            return Err(-1); // table full
        }

        let object_id = self.next_id;
        self.next_id += 1;

        // Copy name into bounded buffer.
        let mut name_buf = [0u8; LINEN_MAX_NAME];
        name_buf[..name.len()].copy_from_slice(name);

        self.objects[slot_idx] = Some(LinenObject {
            object_id,
            kind,
            owner_pd,
            name: name_buf,
            name_len: name.len() as u8,
            ramfs_handle: 0,
            sexfiles_object_id: 0,
            generation: 1,
            flags: 0,
        });

        Ok(object_id)
    }

    /// List objects owned by `caller_pd`, starting from `start_idx`.
    /// Returns the next owned object, or None if no more.
    /// caller_pd == 0 bypasses owner filter (server-internal use).
    pub fn list(&self, caller_pd: u32, start_idx: u8) -> Option<LinenObject> {
        let start = start_idx as usize;
        for i in start..LINEN_MAX_OBJECTS {
            if let Some(obj) = &self.objects[i] {
                if caller_pd == 0 || obj.owner_pd == caller_pd {
                    return Some(*obj);
                }
            }
        }
        None
    }

    /// Get a specific object by ID, with owner validation.
    /// Returns the object on success, or:
    /// - `-3` if not found
    /// - `-6` if caller is not the owner (ERR_PERM_DENIED)
    pub fn get(&self, object_id: u64, caller_pd: u32) -> Result<LinenObject, i64> {
        for slot in self.objects.iter() {
            if let Some(obj) = slot {
                if obj.object_id == object_id {
                    if caller_pd == 0 || obj.owner_pd == caller_pd {
                        return Ok(*obj);
                    } else {
                        return Err(-6); // ERR_PERM_DENIED
                    }
                }
            }
        }
        Err(-3) // not found
    }

    /// Return the object at the given slot index, or None if the slot is empty.
    /// No owner filter — direct slot access for public snapshot rendering.
    pub fn get_at_slot(&self, slot: usize) -> Option<LinenObject> {
        if slot < LINEN_MAX_OBJECTS {
            self.objects[slot]
        } else {
            None
        }
    }

    /// Return the number of objects in the session.
    pub fn count(&self) -> usize {
        let mut c = 0;
        for slot in self.objects.iter() {
            if slot.is_some() {
                c += 1;
            }
        }
        c
    }

    /// Return the number of objects owned by `caller_pd`.
    pub fn count_owned(&self, caller_pd: u32) -> usize {
        let mut c = 0;
        for slot in self.objects.iter() {
            if let Some(obj) = slot {
                if caller_pd == 0 || obj.owner_pd == caller_pd {
                    c += 1;
                }
            }
        }
        c
    }

    /// Mark an object as persisted with a RamFS handle.
    /// Sets flags bit 0 and stores the handle.
    pub fn set_persisted(&mut self, object_id: u64, ramfs_handle: u64) -> Result<(), i64> {
        for slot in self.objects.iter_mut() {
            if let Some(obj) = slot {
                if obj.object_id == object_id {
                    obj.ramfs_handle = ramfs_handle;
                    obj.flags |= 0x01;
                    return Ok(());
                }
            }
        }
        Err(-3) // not found
    }

    /// Bind the global SexFiles-assigned object_id to a Linen object.
    /// Called after OP_RAMFS_OBJECT_ID resolves the authoritative global ID.
    /// 0 is not a valid sexfiles_object_id (RamFS assigns ≥1).
    pub fn set_sexfiles_object_id(&mut self, object_id: u64, sexfiles_id: u64) -> Result<(), i64> {
        for slot in self.objects.iter_mut() {
            if let Some(obj) = slot {
                if obj.object_id == object_id {
                    obj.sexfiles_object_id = sexfiles_id;
                    return Ok(());
                }
            }
        }
        Err(-3)
    }

    /// Bump the generation counter for an object.
    /// Called when metadata is updated.
    pub fn bump_generation(&mut self, object_id: u64) -> Result<u64, i64> {
        for slot in self.objects.iter_mut() {
            if let Some(obj) = slot {
                if obj.object_id == object_id {
                    obj.generation = obj.generation.saturating_add(1);
                    return Ok(obj.generation);
                }
            }
        }
        Err(-3)
    }
}
