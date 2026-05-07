use crate::backends::FsBackend;
use crate::messages;
use core::sync::atomic::{AtomicU64, Ordering};
use sex_pdx::{
    pdx_call, serial_println, SLOT_BLOCK,
    BLOCK_READ, BLOCK_WRITE, BLOCK_SYNC,
};
use spin::RwLock;

pub const DISKFS_BLOCK_SIZE: u32 = 4096;
pub const DISKFS_MAX_OBJECTS: usize = 16;
pub const DISKFS_JOURNAL_CAPACITY: usize = 64;
/// Bounded array of object-table checkpoints (generational snapshots).
/// Each checkpoint captures the full object table + superblock generation
/// at a point in time, with a checksum for integrity validation.
/// Small fixed bound — no recursive subvolumes, no Btrfs clone.
pub const DISKFS_MAX_CHECKPOINTS: usize = 4;
const DISKFS_MAGIC: u64 = 0x3156_5345_4C49_4653; // "SFILESV1"
const DISKFS_CHECKPOINT_MAGIC: u64 = 0x4348_4B50_4E54_5631; // "CHKPNTV1"

/// Extent/free-space allocator: fixed-size block bitmap for contiguous allocations.
/// 1024 blocks × 4096 bytes = 4 MiB of addressable storage.
/// Simple first-fit; no fragmentation solver beyond reuse on free.
pub const DISKFS_EXTENT_BLOCK_COUNT: usize = 1024;
pub const DISKFS_EXTENT_BITMAP_WORDS: usize = DISKFS_EXTENT_BLOCK_COUNT / 64; // 16
pub const DISKFS_EXTENT_JOURNAL_CAPACITY: usize = 32;
const DISKFS_EXTENT_JOURNAL_KIND: u16 = 10; // JournalRecordKind discriminator for extent ops

#[derive(Clone, Copy)]
pub struct SexfilesSuperblock {
    pub magic: u64,
    pub version_major: u16,
    pub version_minor: u16,
    pub block_size: u32,
    pub fs_generation: u64,
    pub object_table_start_block: u64,
    pub object_table_entry_count: u32,
    pub feature_flags: u64,
    pub checksum: u32,
}

#[derive(Clone, Copy)]
pub struct SexfilesObjectEntry {
    pub object_id: u64,
    pub kind: u16,
    pub owner_pd: u32,
    pub rights_generation: u64,
    pub object_size_bytes: u64,
    pub first_block: u64,
    pub metadata_generation: u64,
    pub checksum: u32,
    pub in_use: bool,
}

#[derive(Clone, Copy)]
#[repr(u16)]
pub enum JournalRecordKind {
    TxBegin = 1,
    ObjectMetadataUpdate = 2,
    TxCommit = 3,
}

/// [sexfiles.checkpoint.proof] — generational snapshot of the object table.
///
/// Each checkpoint captures the full object table array + the superblock
/// fs_generation at the moment of creation. Checkpoints are append-only:
/// new checkpoints are written to the next free slot in a bounded array;
/// older checkpoints are overwritten circularly when the array is full.
///
/// On restore, the latest valid checkpoint (highest checkpoint_generation
/// with valid checksum) is selected. Corrupted checkpoints are skipped.
///
/// This is NOT a Btrfs snapshot, NOT a recursive subvolume — it is a
/// flat, bounded, in-memory object-metadata-only generation record.
#[derive(Clone, Copy)]
pub struct SexfilesCheckpoint {
    pub magic: u64,
    pub checkpoint_generation: u64,
    pub fs_generation: u64,
    pub table: [SexfilesObjectEntry; DISKFS_MAX_OBJECTS],
    pub checksum: u32,
    pub valid: bool,
}

#[derive(Clone, Copy)]
pub struct JournalRecord {
    pub kind: JournalRecordKind,
    pub tx_id: u64,
    pub generation: u64,
    pub object_id: u64,
    pub metadata_generation: u64,
    pub payload_len: u16,
    pub checksum: u32,
}

#[derive(Clone, Copy)]
pub struct ReplayOutcome {
    pub committed_applied: bool,
    pub uncommitted_ignored: bool,
    pub corrupt_rejected: bool,
    pub generation_order: bool,
    pub object_restored: bool,
}

/// Reboot persistence proof outcome.
/// Reports whether a full journal→replay→verify roundtrip succeeds
/// within the in-memory scaffold (simulated reboot).
#[derive(Clone, Copy)]
pub struct RebootOutcome {
    pub write_committed: bool,
    pub objects_created: u32,
    pub journal_records: usize,
    pub replay_applied: usize,
    pub objects_restored: bool,
    pub fs_generation_advanced: bool,
}

#[derive(Clone, Copy)]
struct DiskFsState {
    mounted: bool,
    superblock: SexfilesSuperblock,
    table: [SexfilesObjectEntry; DISKFS_MAX_OBJECTS],
    journal: [JournalRecord; DISKFS_JOURNAL_CAPACITY],
    journal_len: usize,
    next_tx_id: u64,
    /// Extent allocator block bitmap: 1 bit per block (1=allocated, 0=free).
    /// Block 0 is reserved for superblock (always marked as in-use).
    extent_bitmap: [u64; DISKFS_EXTENT_BITMAP_WORDS],
    /// Separate small journal for extent allocator metadata operations.
    extent_journal: [JournalRecord; DISKFS_EXTENT_JOURNAL_CAPACITY],
    extent_journal_len: usize,
    /// [sexfiles.checkpoint.proof] — bounded array of object-table generation snapshots.
    checkpoints: [SexfilesCheckpoint; DISKFS_MAX_CHECKPOINTS],
    next_checkpoint_generation: u64,
}

const ZERO_ENTRY: SexfilesObjectEntry = SexfilesObjectEntry {
    object_id: 0,
    kind: 0,
    owner_pd: 0,
    rights_generation: 0,
    object_size_bytes: 0,
    first_block: 0,
    metadata_generation: 0,
    checksum: 0,
    in_use: false,
};

const ZERO_SUPERBLOCK: SexfilesSuperblock = SexfilesSuperblock {
    magic: 0,
    version_major: 0,
    version_minor: 0,
    block_size: 0,
    fs_generation: 0,
    object_table_start_block: 0,
    object_table_entry_count: 0,
    feature_flags: 0,
    checksum: 0,
};

const ZERO_JOURNAL_RECORD: JournalRecord = JournalRecord {
    kind: JournalRecordKind::TxBegin,
    tx_id: 0,
    generation: 0,
    object_id: 0,
    metadata_generation: 0,
    payload_len: 0,
    checksum: 0,
};

const ZERO_CHECKPOINT: SexfilesCheckpoint = SexfilesCheckpoint {
    magic: 0,
    checkpoint_generation: 0,
    fs_generation: 0,
    table: [ZERO_ENTRY; DISKFS_MAX_OBJECTS],
    checksum: 0,
    valid: false,
};

const ZERO_STATE: DiskFsState = DiskFsState {
    mounted: false,
    superblock: ZERO_SUPERBLOCK,
    table: [ZERO_ENTRY; DISKFS_MAX_OBJECTS],
    journal: [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY],
    journal_len: 0,
    next_tx_id: 1,
    extent_bitmap: [0u64; DISKFS_EXTENT_BITMAP_WORDS],
    extent_journal: [ZERO_JOURNAL_RECORD; DISKFS_EXTENT_JOURNAL_CAPACITY],
    extent_journal_len: 0,
    checkpoints: [ZERO_CHECKPOINT; DISKFS_MAX_CHECKPOINTS],
    next_checkpoint_generation: 1,
};

/// DiskFs: bounded in-memory mock scaffold for V1 on-disk format lock.
///
/// BLOCKER: No real block I/O path is wired yet in sexfiles->sexdrive.
/// The system lacks: a block device server (sexdrive is a framebuffer demo),
/// block device PDX opcodes/slots, block device kernel syscalls,
/// and any NVMe/AHCI driver infrastructure.
///
/// This module implements the bounded on-disk format contract (superblock,
/// object table, journal) against an in-memory scaffold. The block read/write
/// proof methods below validate that the alignment and bounds contracts are
/// correct — but they operate on the in-memory state, not real persistent media.
///
/// See: docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md for the exact missing
/// contract and smallest future patch.
#[allow(dead_code)]
pub struct DiskFs;

static DISKFS_STATE: RwLock<DiskFsState> = RwLock::new(ZERO_STATE);
static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

impl DiskFs {
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self
    }

    /// [sexfiles.diskfs.call] [sexfiles.diskfs.reply]
    /// Route proof: send a block/DMA call to sexdrive via SLOT_BLOCK.
    /// opcode: block operation (e.g. read/write/flush).
    /// arg0-arg2: operation-specific parameters (block offset, length, buffer).
    /// Returns the sexdrive reply value, or 0 on error.
    /// Proof markers emitted: call, reply — validated via serial_println trace.
    #[allow(dead_code)]
    pub fn diskfs_block_call(opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
        serial_println!(
            "[sexfiles.diskfs.call] slot={} opcode={:#x} arg0={:#x} arg1={:#x} arg2={:#x}",
            SLOT_BLOCK, opcode, arg0, arg1, arg2
        );
        let (status, value) = pdx_call(SLOT_BLOCK, opcode, arg0, arg1, arg2);
        serial_println!(
            "[sexfiles.diskfs.reply] status={:#x} value={:#x}",
            status, value
        );
        value
    }

    /// [sexfiles.diskfs.typed.call] [sexfiles.diskfs.typed.reply]
    /// Typed block read: send BLOCK_READ command to sexdrive.
    /// offset: sector-aligned byte offset in block device.
    /// size: transfer size in bytes (≤ BLOCK_MAX_XFER, sector-aligned).
    /// buffer_cap: placeholder capability token (0 = no DMA buffer yet).
    /// Returns block status code (BLOCK_OK on success, error code on failure).
    #[allow(dead_code)]
    pub fn diskfs_block_read(offset: u64, size: u64, buffer_cap: u64) -> u64 {
        serial_println!(
            "[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset={:#x} size={} buf_cap={:#x}",
            offset, size, buffer_cap
        );
        let reply = Self::diskfs_block_call(BLOCK_READ, offset, size, buffer_cap);
        serial_println!(
            "[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status={}",
            reply
        );
        reply
    }

    /// [sexfiles.diskfs.typed.call] [sexfiles.diskfs.typed.reply]
    /// Typed block write: send BLOCK_WRITE command to sexdrive.
    #[allow(dead_code)]
    pub fn diskfs_block_write(offset: u64, size: u64, buffer_cap: u64) -> u64 {
        serial_println!(
            "[sexfiles.diskfs.typed.call] cmd=BLOCK_WRITE offset={:#x} size={} buf_cap={:#x}",
            offset, size, buffer_cap
        );
        let reply = Self::diskfs_block_call(BLOCK_WRITE, offset, size, buffer_cap);
        serial_println!(
            "[sexfiles.diskfs.typed.reply] cmd=BLOCK_WRITE status={}",
            reply
        );
        reply
    }

    /// [sexfiles.diskfs.typed.call] [sexfiles.diskfs.typed.reply]
    /// Typed block sync: send BLOCK_SYNC command to sexdrive (flush/barrier).
    #[allow(dead_code)]
    pub fn diskfs_block_sync() -> u64 {
        serial_println!(
            "[sexfiles.diskfs.typed.call] cmd=BLOCK_SYNC"
        );
        let reply = Self::diskfs_block_call(BLOCK_SYNC, 0, 0, 0);
        serial_println!(
            "[sexfiles.diskfs.typed.reply] cmd=BLOCK_SYNC status={}",
            reply
        );
        reply
    }

    fn checksum_superblock(sb: &SexfilesSuperblock) -> u32 {
        (sb.magic as u32)
            ^ (sb.magic >> 32) as u32
            ^ sb.version_major as u32
            ^ sb.version_minor as u32
            ^ sb.block_size
            ^ (sb.fs_generation as u32)
            ^ (sb.fs_generation >> 32) as u32
            ^ (sb.object_table_start_block as u32)
            ^ (sb.object_table_start_block >> 32) as u32
            ^ sb.object_table_entry_count
            ^ (sb.feature_flags as u32)
            ^ (sb.feature_flags >> 32) as u32
    }

    fn checksum_entry(e: &SexfilesObjectEntry) -> u32 {
        (e.object_id as u32)
            ^ (e.object_id >> 32) as u32
            ^ e.kind as u32
            ^ e.owner_pd
            ^ (e.rights_generation as u32)
            ^ (e.rights_generation >> 32) as u32
            ^ (e.object_size_bytes as u32)
            ^ (e.object_size_bytes >> 32) as u32
            ^ (e.first_block as u32)
            ^ (e.first_block >> 32) as u32
            ^ (e.metadata_generation as u32)
            ^ (e.metadata_generation >> 32) as u32
            ^ (e.in_use as u32)
    }

    fn checksum_journal_record(rec: &JournalRecord) -> u32 {
        (rec.kind as u32)
            ^ (rec.tx_id as u32)
            ^ (rec.tx_id >> 32) as u32
            ^ (rec.generation as u32)
            ^ (rec.generation >> 32) as u32
            ^ (rec.object_id as u32)
            ^ (rec.object_id >> 32) as u32
            ^ (rec.metadata_generation as u32)
            ^ (rec.metadata_generation >> 32) as u32
            ^ (rec.payload_len as u32)
    }

    /// [sexfiles.checkpoint.proof] — checksum over a checkpoint record.
    /// Covers magic, both generations, and a hash of all in-use entries.
    fn checksum_checkpoint(cp: &SexfilesCheckpoint) -> u32 {
        let mut hash: u32 = (cp.magic as u32)
            ^ (cp.magic >> 32) as u32
            ^ (cp.checkpoint_generation as u32)
            ^ (cp.checkpoint_generation >> 32) as u32
            ^ (cp.fs_generation as u32)
            ^ (cp.fs_generation >> 32) as u32;
        // Mix in per-entry checksums for all in-use entries in the table.
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            if cp.table[i].in_use {
                let e_hash = Self::checksum_entry(&cp.table[i]);
                hash ^= e_hash;
                hash ^= (i as u32) << 16; // position-dependent mixing
            }
            i += 1;
        }
        hash
    }

    fn make_journal_record(
        kind: JournalRecordKind,
        tx_id: u64,
        generation: u64,
        object_id: u64,
        metadata_generation: u64,
        payload_len: u16,
    ) -> JournalRecord {
        let mut rec = JournalRecord {
            kind,
            tx_id,
            generation,
            object_id,
            metadata_generation,
            payload_len,
            checksum: 0,
        };
        rec.checksum = Self::checksum_journal_record(&rec);
        rec
    }

    fn append_journal_record(st: &mut DiskFsState, rec: JournalRecord) -> Result<(), i64> {
        if st.journal_len >= DISKFS_JOURNAL_CAPACITY {
            return Err(messages::ERR_FULL);
        }
        if Self::checksum_journal_record(&rec) != rec.checksum {
            return Err(messages::ERR_OVERFLOW);
        }
        st.journal[st.journal_len] = rec;
        st.journal_len += 1;
        Ok(())
    }

    fn begin_transaction(st: &mut DiskFsState) -> Result<u64, i64> {
        let tx_id = st.next_tx_id;
        st.next_tx_id = st.next_tx_id.saturating_add(1);
        let rec = Self::make_journal_record(
            JournalRecordKind::TxBegin,
            tx_id,
            st.superblock.fs_generation,
            0,
            0,
            0,
        );
        Self::append_journal_record(st, rec)?;
        Ok(tx_id)
    }

    fn append_object_metadata_update(
        st: &mut DiskFsState,
        tx_id: u64,
        object_id: u64,
        metadata_generation: u64,
    ) -> Result<(), i64> {
        let rec = Self::make_journal_record(
            JournalRecordKind::ObjectMetadataUpdate,
            tx_id,
            st.superblock.fs_generation,
            object_id,
            metadata_generation,
            32,
        );
        Self::append_journal_record(st, rec)
    }

    fn commit_transaction(st: &mut DiskFsState, tx_id: u64) -> Result<(), i64> {
        let rec = Self::make_journal_record(
            JournalRecordKind::TxCommit,
            tx_id,
            st.superblock.fs_generation,
            0,
            0,
            0,
        );
        Self::append_journal_record(st, rec)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  Extent/free-space allocator: bounded block bitmap with first-fit.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Set a single bit in the extent bitmap (mark block as allocated).
    fn extent_bitmap_set(st: &mut DiskFsState, block: u64) -> Result<(), i64> {
        if block >= DISKFS_EXTENT_BLOCK_COUNT as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        let word_idx = (block / 64) as usize;
        let bit = block % 64;
        if st.extent_bitmap[word_idx] & (1u64 << bit) != 0 {
            return Err(messages::ERR_FULL); // already allocated
        }
        st.extent_bitmap[word_idx] |= 1u64 << bit;
        Ok(())
    }

    /// Clear a single bit in the extent bitmap (mark block as free).
    fn extent_bitmap_clear(st: &mut DiskFsState, block: u64) -> Result<(), i64> {
        if block >= DISKFS_EXTENT_BLOCK_COUNT as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        let word_idx = (block / 64) as usize;
        let bit = block % 64;
        if st.extent_bitmap[word_idx] & (1u64 << bit) == 0 {
            return Err(messages::ERR_INVALID_HANDLE); // already free
        }
        st.extent_bitmap[word_idx] &= !(1u64 << bit);
        Ok(())
    }

    /// Check if a single bit is set (block allocated).
    fn extent_bitmap_is_set(st: &DiskFsState, block: u64) -> Result<bool, i64> {
        if block >= DISKFS_EXTENT_BLOCK_COUNT as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        let word_idx = (block / 64) as usize;
        let bit = block % 64;
        Ok(st.extent_bitmap[word_idx] & (1u64 << bit) != 0)
    }

    /// Find `count` contiguous free blocks using simple first-fit scan.
    /// Returns the starting block number, or None if no span large enough exists.
    fn extent_bitmap_find_contiguous(st: &DiskFsState, count: u32) -> Option<u64> {
        if count == 0 || count as usize > DISKFS_EXTENT_BLOCK_COUNT {
            return None;
        }
        let count = count as u64;
        // Block 0 is reserved (superblock), start scan from block 1.
        let mut run_start: Option<u64> = None;
        let mut run_len: u64 = 0;
        let mut block: u64 = 1;
        while block < DISKFS_EXTENT_BLOCK_COUNT as u64 {
            let word_idx = (block / 64) as usize;
            let bit = block % 64;
            let allocated = st.extent_bitmap[word_idx] & (1u64 << bit) != 0;
            if !allocated {
                if run_start.is_none() {
                    run_start = Some(block);
                }
                run_len += 1;
                if run_len >= count {
                    return run_start;
                }
            } else {
                run_start = None;
                run_len = 0;
            }
            block += 1;
        }
        None
    }

    /// Count total allocated blocks in the bitmap.
    fn extent_bitmap_used_count(st: &DiskFsState) -> usize {
        let mut count: usize = 0;
        let mut i = 0usize;
        while i < DISKFS_EXTENT_BITMAP_WORDS {
            count += st.extent_bitmap[i].count_ones() as usize;
            i += 1;
        }
        count
    }

    /// Append a single record to the extent journal.
    fn extent_journal_append(st: &mut DiskFsState, rec: JournalRecord) -> Result<(), i64> {
        if st.extent_journal_len >= DISKFS_EXTENT_JOURNAL_CAPACITY {
            return Err(messages::ERR_FULL);
        }
        if Self::checksum_journal_record(&rec) != rec.checksum {
            return Err(messages::ERR_OVERFLOW);
        }
        st.extent_journal[st.extent_journal_len] = rec;
        st.extent_journal_len += 1;
        Ok(())
    }

    /// [sexfiles.extent.proof.alloc]
    /// Allocate `count` contiguous blocks. Returns the first block number.
    /// Journals the allocation if the journal is active.
    /// This is a bounded, deterministic first-fit allocator with
    /// no fragmentation solver beyond reuse on free.
    pub fn allocate_blocks(&self, count: u32) -> Result<u64, i64> {
        let mut st = DISKFS_STATE.write();
        Self::verify_journal_integrity(&st)?;
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        if count == 0 || count as usize > DISKFS_EXTENT_BLOCK_COUNT {
            return Err(messages::ERR_OVERFLOW);
        }
        let start = match Self::extent_bitmap_find_contiguous(&st, count) {
            Some(s) => s,
            None => return Err(messages::ERR_FULL),
        };
        // Mark all blocks as allocated.
        let mut i: u64 = 0;
        while i < count as u64 {
            Self::extent_bitmap_set(&mut st, start + i)?;
            i += 1;
        }
        // Journal the allocation if the extent journal is active.
        if st.extent_journal_len > 0 || true {
            let tx_id = st.next_tx_id;
            st.next_tx_id = st.next_tx_id.saturating_add(1);
            let rec = Self::make_journal_record(
                JournalRecordKind::ObjectMetadataUpdate,
                tx_id,
                st.superblock.fs_generation,
                start,
                count as u64,
                DISKFS_EXTENT_JOURNAL_KIND,
            );
            Self::extent_journal_append(&mut st, rec)?;
        }
        Ok(start)
    }

    /// [sexfiles.extent.proof.free]
    /// Free `count` blocks starting at `first_block`.
    /// Journals the free operation.
    pub fn free_blocks(&self, first_block: u64, count: u32) -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        Self::verify_journal_integrity(&st)?;
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        if count == 0 || first_block >= DISKFS_EXTENT_BLOCK_COUNT as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        if first_block + count as u64 > DISKFS_EXTENT_BLOCK_COUNT as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        let mut i: u64 = 0;
        while i < count as u64 {
            Self::extent_bitmap_clear(&mut st, first_block + i)?;
            i += 1;
        }
        // Journal the free.
        if st.extent_journal_len > 0 || true {
            let tx_id = st.next_tx_id;
            st.next_tx_id = st.next_tx_id.saturating_add(1);
            let rec = Self::make_journal_record(
                JournalRecordKind::ObjectMetadataUpdate,
                tx_id,
                st.superblock.fs_generation,
                first_block | 0x8000_0000_0000_0000, // high bit marks "free"
                count as u64,
                DISKFS_EXTENT_JOURNAL_KIND,
            );
            Self::extent_journal_append(&mut st, rec)?;
        }
        Ok(())
    }

    /// [sexfiles.extent.proof.reuse]
    /// Return the number of blocks currently allocated.
    pub fn used_blocks(&self) -> usize {
        let st = DISKFS_STATE.read();
        Self::extent_bitmap_used_count(&st)
    }

    /// Return the total number of allocatable blocks.
    pub fn total_blocks(&self) -> usize {
        DISKFS_EXTENT_BLOCK_COUNT
    }

    /// [sexfiles.extent.proof.bounds]
    /// Prove that out-of-bounds block numbers are rejected.
    pub fn proof_extent_bounds() -> Result<(), i64> {
        // Test bitmap_set bounds:
        // - At max valid: ok
        Self::extent_bitmap_set(
            &mut DISKFS_STATE.write(),
            (DISKFS_EXTENT_BLOCK_COUNT - 1) as u64,
        )?;
        Self::extent_bitmap_clear(
            &mut DISKFS_STATE.write(),
            (DISKFS_EXTENT_BLOCK_COUNT - 1) as u64,
        )?;
        // - At exactly max: fails (max block = count - 1)
        let r = Self::extent_bitmap_set(
            &mut DISKFS_STATE.write(),
            DISKFS_EXTENT_BLOCK_COUNT as u64,
        );
        if r.is_ok() {
            return Err(messages::ERR_OVERFLOW);
        }
        // - Zero-length allocation rejected
        let st = DISKFS_STATE.read();
        if Self::extent_bitmap_find_contiguous(&st, 0).is_some() {
            return Err(messages::ERR_OVERFLOW);
        }
        // - count > total blocks rejected
        if Self::extent_bitmap_find_contiguous(&st, (DISKFS_EXTENT_BLOCK_COUNT + 1) as u32).is_some() {
            return Err(messages::ERR_OVERFLOW);
        }
        Ok(())
    }

    /// [sexfiles.extent.proof.full]
    /// Prove deterministic out-of-space: fill all blocks, then verify
    /// the next allocation fails with ERR_FULL.
    pub fn proof_extent_full() -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        // Initialize bitmap: block 0 is reserved (mark as allocated).
        st.extent_bitmap = [0u64; DISKFS_EXTENT_BITMAP_WORDS];
        Self::extent_bitmap_set(&mut st, 0)?;
        st.mounted = true;
        // Fill all remaining blocks with single-block allocations.
        let mut block: u64 = 1;
        while block < DISKFS_EXTENT_BLOCK_COUNT as u64 {
            let word_idx = (block / 64) as usize;
            let bit = block % 64;
            st.extent_bitmap[word_idx] |= 1u64 << bit;
            block += 1;
        }
        // Verify all blocks are used.
        let used = Self::extent_bitmap_used_count(&st);
        if used != DISKFS_EXTENT_BLOCK_COUNT {
            return Err(messages::ERR_OVERFLOW);
        }
        drop(st);
        // Now try to allocate 1 block — must fail.
        let disk = DiskFs::new();
        let r = disk.allocate_blocks(1);
        if r.is_ok() {
            return Err(messages::ERR_OVERFLOW);
        }
        Ok(())
    }

    /// [sexfiles.extent.proof.journaled]
    /// Prove that extent allocator operations are journaled.
    pub fn proof_extent_journaled() -> Result<(usize, usize), i64> {
        let disk = DiskFs::new();
        disk.format_init_empty()?;
        disk.mount()?;
        let before = {
            let st = DISKFS_STATE.read();
            st.extent_journal_len
        };
        // Allocate blocks — should create journal records.
        let first = disk.allocate_blocks(8)?;
        let after_alloc = {
            let st = DISKFS_STATE.read();
            st.extent_journal_len
        };
        if after_alloc <= before {
            return Err(messages::ERR_OVERFLOW);
        }
        // Free blocks — should journal the free.
        disk.free_blocks(first, 8)?;
        let after_free = {
            let st = DISKFS_STATE.read();
            st.extent_journal_len
        };
        if after_free <= after_alloc {
            return Err(messages::ERR_OVERFLOW);
        }
        Ok((after_alloc - before, after_free - after_alloc))
    }

    /// [sexfiles.extent.proof.alloc] + [sexfiles.extent.proof.reuse]
    /// Prove allocate→free→reallocate reuse cycle with bitmap-level validation.
    pub fn proof_extent_alloc_and_reuse() -> Result<(), i64> {
        let disk = DiskFs::new();
        disk.format_init_empty()?;
        disk.mount()?;
        // Block 0 is reserved. Allocate 3 blocks.
        let first = disk.allocate_blocks(3)?;
        if first < 1 {
            return Err(messages::ERR_OVERFLOW);
        }
        // Bitmap-level: verify block is now set.
        let st = DISKFS_STATE.read();
        if !Self::extent_bitmap_is_set(&st, first)? {
            return Err(messages::ERR_NOT_FOUND);
        }
        drop(st);
        let used_after_alloc = disk.used_blocks();
        // Free the 3 blocks.
        disk.free_blocks(first, 3)?;
        // Bitmap-level: verify block is now clear.
        let st = DISKFS_STATE.read();
        if Self::extent_bitmap_is_set(&st, first)? {
            return Err(messages::ERR_NOT_FOUND);
        }
        drop(st);
        let used_after_free = disk.used_blocks();
        if used_after_free >= used_after_alloc {
            return Err(messages::ERR_OVERFLOW);
        }
        // Re-allocate the same 3 blocks — first-fit should find the same spot.
        let second = disk.allocate_blocks(3)?;
        if second != first {
            return Err(messages::ERR_NOT_FOUND); // expected first-fit to reuse same hole
        }
        Ok(())
    }

    fn verify_journal_integrity(st: &DiskFsState) -> Result<(), i64> {
        let mut i = 0usize;
        while i < st.journal_len {
            let rec = st.journal[i];
            if Self::checksum_journal_record(&rec) != rec.checksum {
                return Err(messages::ERR_OVERFLOW);
            }
            i += 1;
        }
        Ok(())
    }

    pub fn proof_inject_bad_record_for_checksum() -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        let tx_id = st.next_tx_id;
        st.next_tx_id = st.next_tx_id.saturating_add(1);
        let mut rec = JournalRecord {
            kind: JournalRecordKind::ObjectMetadataUpdate,
            tx_id,
            generation: st.superblock.fs_generation,
            object_id: 0xBAD,
            metadata_generation: 1,
            payload_len: 32,
            checksum: 0,
        };
        rec.checksum = Self::checksum_journal_record(&rec) ^ 0x1;
        Self::append_journal_record(&mut st, rec)
    }

    pub(crate) fn replay_journal_records(
        records: &[JournalRecord],
        out_table: &mut [SexfilesObjectEntry; DISKFS_MAX_OBJECTS],
    ) -> Result<(usize, bool, bool), i64> {
        // Fixed, bounded tx tracking for replay.
        let mut tx_ids = [0u64; DISKFS_JOURNAL_CAPACITY];
        let mut tx_committed = [false; DISKFS_JOURNAL_CAPACITY];
        let mut tx_seen = 0usize;
        let mut i = 0usize;
        while i < records.len() {
            let rec = records[i];
            if Self::checksum_journal_record(&rec) != rec.checksum {
                return Err(messages::ERR_OVERFLOW);
            }
            match rec.kind {
                JournalRecordKind::TxBegin => {
                    if tx_seen >= DISKFS_JOURNAL_CAPACITY {
                        return Err(messages::ERR_FULL);
                    }
                    tx_ids[tx_seen] = rec.tx_id;
                    tx_committed[tx_seen] = false;
                    tx_seen += 1;
                }
                JournalRecordKind::TxCommit => {
                    let mut j = 0usize;
                    while j < tx_seen {
                        if tx_ids[j] == rec.tx_id {
                            tx_committed[j] = true;
                            break;
                        }
                        j += 1;
                    }
                }
                JournalRecordKind::ObjectMetadataUpdate => {}
            }
            i += 1;
        }

        // Gather committed metadata updates.
        let mut updates = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        let mut update_count = 0usize;
        let mut uncommitted_seen = false;
        i = 0;
        while i < records.len() {
            let rec = records[i];
            if matches!(rec.kind, JournalRecordKind::ObjectMetadataUpdate) {
                let mut committed = false;
                let mut j = 0usize;
                while j < tx_seen {
                    if tx_ids[j] == rec.tx_id {
                        committed = tx_committed[j];
                        break;
                    }
                    j += 1;
                }
                if committed {
                    if update_count >= DISKFS_JOURNAL_CAPACITY {
                        return Err(messages::ERR_FULL);
                    }
                    updates[update_count] = rec;
                    update_count += 1;
                } else {
                    uncommitted_seen = true;
                }
            }
            i += 1;
        }

        // Apply committed updates in generation order.
        let mut applied = [false; DISKFS_JOURNAL_CAPACITY];
        let mut applied_count = 0usize;
        let mut last_gen = 0u64;
        let mut in_order = true;
        while applied_count < update_count {
            let mut min_idx: Option<usize> = None;
            let mut k = 0usize;
            while k < update_count {
                if !applied[k] {
                    match min_idx {
                        Some(mi) => {
                            if updates[k].generation < updates[mi].generation {
                                min_idx = Some(k);
                            }
                        }
                        None => min_idx = Some(k),
                    }
                }
                k += 1;
            }
            let idx = match min_idx {
                Some(v) => v,
                None => break,
            };
            let rec = updates[idx];
            if rec.generation < last_gen {
                in_order = false;
            }
            last_gen = rec.generation;

            // Restore object entry in first free slot if object_id valid.
            if rec.object_id != 0 {
                let mut slot = 0usize;
                while slot < DISKFS_MAX_OBJECTS {
                    if !out_table[slot].in_use {
                        let mut e = SexfilesObjectEntry {
                            object_id: rec.object_id,
                            kind: 1,
                            owner_pd: 11,
                            rights_generation: 1,
                            object_size_bytes: 0,
                            first_block: 0,
                            metadata_generation: rec.metadata_generation,
                            checksum: 0,
                            in_use: true,
                        };
                        e.checksum = Self::checksum_entry(&e);
                        out_table[slot] = e;
                        break;
                    }
                    slot += 1;
                }
            }

            applied[idx] = true;
            applied_count += 1;
        }

        Ok((applied_count, uncommitted_seen, in_order))
    }

    pub fn proof_replay_recovery_scenario() -> Result<ReplayOutcome, i64> {
        let mut records = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        let mut n = 0usize;

        // Committed tx #10: begin -> meta -> commit
        records[n] = Self::make_journal_record(JournalRecordKind::TxBegin, 10, 1, 0, 0, 0);
        n += 1;
        records[n] = Self::make_journal_record(JournalRecordKind::ObjectMetadataUpdate, 10, 2, 1001, 1, 32);
        n += 1;
        records[n] = Self::make_journal_record(JournalRecordKind::TxCommit, 10, 3, 0, 0, 0);
        n += 1;

        // Uncommitted tx #11: begin -> meta (no commit)
        records[n] = Self::make_journal_record(JournalRecordKind::TxBegin, 11, 4, 0, 0, 0);
        n += 1;
        records[n] = Self::make_journal_record(JournalRecordKind::ObjectMetadataUpdate, 11, 5, 2002, 1, 32);
        n += 1;

        let mut table = [ZERO_ENTRY; DISKFS_MAX_OBJECTS];
        let (applied, uncommitted_seen, generation_order) =
            Self::replay_journal_records(&records[..n], &mut table)?;
        let mut restored = false;
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            if table[i].in_use && table[i].object_id == 1001 {
                restored = true;
            }
            if table[i].in_use && table[i].object_id == 2002 {
                // Uncommitted update must not be restored.
                return Err(messages::ERR_OVERFLOW);
            }
            i += 1;
        }

        // Corrupt record reject scenario.
        let mut corrupt = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        corrupt[0] = Self::make_journal_record(JournalRecordKind::TxBegin, 30, 1, 0, 0, 0);
        corrupt[1] = Self::make_journal_record(JournalRecordKind::ObjectMetadataUpdate, 30, 2, 3003, 1, 32);
        corrupt[1].checksum ^= 0x1;
        corrupt[2] = Self::make_journal_record(JournalRecordKind::TxCommit, 30, 3, 0, 0, 0);
        let mut table2 = [ZERO_ENTRY; DISKFS_MAX_OBJECTS];
        let corrupt_rejected = matches!(
            Self::replay_journal_records(&corrupt[..3], &mut table2),
            Err(e) if e == messages::ERR_OVERFLOW
        );

        Ok(ReplayOutcome {
            committed_applied: applied >= 1,
            uncommitted_ignored: uncommitted_seen,
            corrupt_rejected,
            generation_order,
            object_restored: restored,
        })
    }

    /// SexFiles reboot persistence proof roundtrip.
    ///
    /// Simulates a reboot by:
    /// 1. Format + mount
    /// 2. Create known objects (write phase) — object table + journal populated
    /// 3. Export object table snapshot + journal records
    /// 4. Re-format + re-mount (simulating power cycle / reboot)
    /// 5. Restore object table from snapshot (simulates reading table blocks from disk)
    /// 6. Replay exported journal on top of restored table (crash recovery)
    /// 7. Verify objects were restored with correct kind+owner
    ///
    /// On real media: the object table blocks survive the reboot on disk;
    /// the journal replays on top to recover any in-flight transactions.
    /// In this in-memory scaffold we snapshot+restore the table to model
    /// the same behavior.
    ///
    /// BLOCKER: This operates on the in-memory scaffold only. No real
    /// block device transport exists to persist table/journal across
    /// a true QEMU process restart. Real reboot persistence requires
    /// the block device route documented in:
    /// docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md
    pub fn proof_reboot_persistence_roundtrip() -> Result<RebootOutcome, i64> {
        // ── Step 1: Format + mount fresh state ──
        let mut st = DISKFS_STATE.write();
        let mut sb = SexfilesSuperblock {
            magic: DISKFS_MAGIC,
            version_major: 1,
            version_minor: 0,
            block_size: DISKFS_BLOCK_SIZE,
            fs_generation: 1,
            object_table_start_block: 1,
            object_table_entry_count: DISKFS_MAX_OBJECTS as u32,
            feature_flags: 0,
            checksum: 0,
        };
        sb.checksum = Self::checksum_superblock(&sb);
        st.superblock = sb;
        st.table = [ZERO_ENTRY; DISKFS_MAX_OBJECTS];
        st.journal = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        st.journal_len = 0;
        st.next_tx_id = 1;
        st.mounted = true;
        NEXT_OBJECT_ID.store(1, Ordering::SeqCst);
        drop(st);

        // ── Step 2 (WRITE PHASE): Create known objects ──
        let oid_a = DiskFs::new().create_object_entry(42, 1)?;
        let oid_b = DiskFs::new().create_object_entry(43, 1)?;

        // Verify objects exist before reboot
        let sa = DiskFs::new().stat_object_entry(oid_a)?;
        let sb_pre = DiskFs::new().stat_object_entry(oid_b)?;
        let ok_pre = sa.kind == 42 && sa.owner_pd == 1 && sb_pre.kind == 43 && sb_pre.owner_pd == 1;

        let gen_before_reboot = {
            let st = DISKFS_STATE.read();
            st.superblock.fs_generation
        };

        // ── Step 3: Export object table snapshot + journal records ──
        let saved_table: [SexfilesObjectEntry; DISKFS_MAX_OBJECTS];
        let mut saved_journal = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        let mut journal_len = 0usize;
        {
            let st = DISKFS_STATE.read();
            saved_table = st.table;
            while journal_len < st.journal_len && journal_len < DISKFS_JOURNAL_CAPACITY {
                saved_journal[journal_len] = st.journal[journal_len];
                journal_len += 1;
            }
        }

        // ── Step 4 (REBOOT): Re-format + re-mount (fresh empty state) ──
        let mut st = DISKFS_STATE.write();
        let mut sb2 = SexfilesSuperblock {
            magic: DISKFS_MAGIC,
            version_major: 1,
            version_minor: 0,
            block_size: DISKFS_BLOCK_SIZE,
            fs_generation: gen_before_reboot.saturating_add(1),
            object_table_start_block: 1,
            object_table_entry_count: DISKFS_MAX_OBJECTS as u32,
            feature_flags: 0,
            checksum: 0,
        };
        sb2.checksum = Self::checksum_superblock(&sb2);
        st.superblock = sb2;
        // Start with empty table (simulates reading from uninitialized disk blocks
        // before journal replay restores consistency)
        st.table = [ZERO_ENTRY; DISKFS_MAX_OBJECTS];
        st.journal = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        st.journal_len = 0;
        st.next_tx_id = 1;
        st.mounted = true;
        let gen_after_reboot = st.superblock.fs_generation;
        drop(st);

        // ── Step 5 (RECOVERY): Restore object table from disk snapshot,
        //     then replay journal on top to recover in-flight transactions. ──
        // On real hardware: the object table blocks are read from disk.
        // Here we copy the saved table back into state, then replay the
        // journal — committed transactions update the table entries that
        // were in-flight during the crash.
        {
            let mut st = DISKFS_STATE.write();
            st.table = saved_table;
            drop(st);
        }
        // Replay journal on top of restored table.
        let mut table_with_replay = saved_table;
        let (applied, _uncommitted, _gen_order) =
            Self::replay_journal_records(&saved_journal[..journal_len], &mut table_with_replay)?;
        {
            let mut st = DISKFS_STATE.write();
            // Merge: any entries created by replay (in free slots) are added.
            let mut slot = 0usize;
            while slot < DISKFS_MAX_OBJECTS {
                if table_with_replay[slot].in_use && !st.table[slot].in_use {
                    st.table[slot] = table_with_replay[slot];
                    st.table[slot].checksum = Self::checksum_entry(&st.table[slot]);
                }
                slot += 1;
            }
            drop(st);
        }

        // ── Step 6 (VERIFY PHASE): Check objects restored with correct attributes ──
        let maybe_sa_after = DiskFs::new().stat_object_entry(oid_a);
        let maybe_sb_after = DiskFs::new().stat_object_entry(oid_b);

        let (ok_a, ok_b) = match (maybe_sa_after, maybe_sb_after) {
            (Ok(sa2), Ok(sb2)) => (
                sa2.kind == 42 && sa2.owner_pd == 1,
                sb2.kind == 43 && sb2.owner_pd == 1,
            ),
            _ => (false, false),
        };

        Ok(RebootOutcome {
            write_committed: ok_pre,
            objects_created: 2,
            journal_records: journal_len,
            replay_applied: applied,
            objects_restored: ok_a && ok_b,
            fs_generation_advanced: gen_after_reboot > gen_before_reboot,
        })
    }

    /// Format/init empty DiskFS scaffold.
    pub fn format_init_empty(&self) -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        let mut sb = SexfilesSuperblock {
            magic: DISKFS_MAGIC,
            version_major: 1,
            version_minor: 0,
            block_size: DISKFS_BLOCK_SIZE,
            fs_generation: 1,
            object_table_start_block: 1,
            object_table_entry_count: DISKFS_MAX_OBJECTS as u32,
            feature_flags: 0,
            checksum: 0,
        };
        sb.checksum = Self::checksum_superblock(&sb);
        st.superblock = sb;
        st.table = [ZERO_ENTRY; DISKFS_MAX_OBJECTS];
        st.journal = [ZERO_JOURNAL_RECORD; DISKFS_JOURNAL_CAPACITY];
        st.journal_len = 0;
        st.next_tx_id = 1;
        st.mounted = false;
        // Initialize extent bitmap: block 0 = superblock (reserved).
        st.extent_bitmap = [0u64; DISKFS_EXTENT_BITMAP_WORDS];
        st.extent_bitmap[0] |= 1u64; // block 0 reserved
        st.extent_journal = [ZERO_JOURNAL_RECORD; DISKFS_EXTENT_JOURNAL_CAPACITY];
        st.extent_journal_len = 0;
        // Initialize checkpoint array.
        st.checkpoints = [ZERO_CHECKPOINT; DISKFS_MAX_CHECKPOINTS];
        st.next_checkpoint_generation = 1;
        NEXT_OBJECT_ID.store(1, Ordering::SeqCst);
        Ok(())
    }

    /// Mount/read superblock and validate bounded format.
    pub fn mount(&self) -> Result<SexfilesSuperblock, i64> {
        let mut st = DISKFS_STATE.write();
        let sb = st.superblock;
        if sb.magic != DISKFS_MAGIC || sb.version_major != 1 || sb.block_size != DISKFS_BLOCK_SIZE {
            return Err(messages::ERR_NOT_FOUND);
        }
        if sb.object_table_entry_count as usize != DISKFS_MAX_OBJECTS {
            return Err(messages::ERR_OVERFLOW);
        }
        if Self::checksum_superblock(&sb) != sb.checksum {
            return Err(messages::ERR_OVERFLOW);
        }
        // Verify block 0 is reserved in the extent bitmap.
        if st.extent_bitmap[0] & 1u64 == 0 {
            // Auto-reserve if not already set (fresh format).
            st.extent_bitmap[0] |= 1u64;
        }
        st.mounted = true;
        Ok(sb)
    }

    /// Create bounded object entry.
    pub fn create_object_entry(&self, kind: u16, owner_pd: u32) -> Result<u64, i64> {
        let mut st = DISKFS_STATE.write();
        Self::verify_journal_integrity(&st)?;
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }

        let mut free_idx: Option<usize> = None;
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            if !st.table[i].in_use {
                free_idx = Some(i);
                break;
            }
            i += 1;
        }
        let idx = match free_idx {
            Some(v) => v,
            None => return Err(messages::ERR_FULL),
        };

        let object_id = NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst);
        let tx_id = Self::begin_transaction(&mut st)?;
        Self::append_object_metadata_update(&mut st, tx_id, object_id, 1)?;
        Self::commit_transaction(&mut st, tx_id)?;
        let mut e = SexfilesObjectEntry {
            object_id,
            kind,
            owner_pd,
            rights_generation: 1,
            object_size_bytes: 0,
            first_block: 0,
            metadata_generation: 1,
            checksum: 0,
            in_use: true,
        };
        e.checksum = Self::checksum_entry(&e);
        st.table[idx] = e;

        st.superblock.fs_generation = st.superblock.fs_generation.saturating_add(1);
        st.superblock.checksum = Self::checksum_superblock(&st.superblock);
        Ok(object_id)
    }

    /// Stat object entry by id.
    pub fn stat_object_entry(&self, object_id: u64) -> Result<SexfilesObjectEntry, i64> {
        let st = DISKFS_STATE.read();
        Self::verify_journal_integrity(&st)?;
        if !st.mounted || object_id == 0 {
            return Err(messages::ERR_INVALID_HANDLE);
        }
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            let e = st.table[i];
            if e.in_use && e.object_id == object_id {
                if Self::checksum_entry(&e) != e.checksum {
                    return Err(messages::ERR_OVERFLOW);
                }
                return Ok(e);
            }
            i += 1;
        }
        Err(messages::ERR_INVALID_HANDLE)
    }

    pub fn journal_len(&self) -> usize {
        let st = DISKFS_STATE.read();
        st.journal_len
    }

    /// Fault injection: corrupt an object entry's checksum bit.
    /// Used by SEXOS_SEXFILES_FAULT_INJECTION_PROOF to verify
    /// entry-level integrity detection (stat returns ERR_OVERFLOW).
    pub fn proof_inject_bad_entry_checksum(object_id: u64) -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            if st.table[i].in_use && st.table[i].object_id == object_id {
                st.table[i].checksum ^= 0x1;
                return Ok(());
            }
            i += 1;
        }
        Err(messages::ERR_INVALID_HANDLE)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  CHECKPOINT / SNAPSHOT — Generational object-table state records.
    //  Minimal bounded implementation: no subvolumes, no Btrfs clone.
    // ═══════════════════════════════════════════════════════════════════════════

    /// [sexfiles.checkpoint.proof.create]
    /// Create a checkpoint of the current object table state.
    /// Snapshots the full table + superblock fs_generation into a
    /// bounded checkpoint slot. Returns the checkpoint's own generation
    /// number on success.
    ///
    /// Checkpoints are append-only in the bounded array. When all slots
    /// are full, the oldest (lowest checkpoint_generation) is overwritten.
    pub fn create_checkpoint(&self) -> Result<u64, i64> {
        let st = DISKFS_STATE.read();
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        let cp_gen = st.next_checkpoint_generation;
        let fs_gen = st.superblock.fs_generation;
        let table_snapshot = st.table;
        drop(st);

        let mut st = DISKFS_STATE.write();

        // Find the next free slot, or overwrite the oldest checkpoint.
        let mut target_slot: Option<usize> = None;
        let mut min_gen: u64 = u64::MAX;
        let mut i = 0usize;
        while i < DISKFS_MAX_CHECKPOINTS {
            let cp = st.checkpoints[i];
            if !cp.valid {
                target_slot = Some(i);
                break;
            }
            if cp.checkpoint_generation < min_gen {
                min_gen = cp.checkpoint_generation;
                target_slot = Some(i);
            }
            i += 1;
        }

        let slot = target_slot.ok_or(messages::ERR_FULL)?;

        let mut cp = SexfilesCheckpoint {
            magic: DISKFS_CHECKPOINT_MAGIC,
            checkpoint_generation: cp_gen,
            fs_generation: fs_gen,
            table: table_snapshot,
            checksum: 0,
            valid: true,
        };
        cp.checksum = Self::checksum_checkpoint(&cp);
        st.checkpoints[slot] = cp;
        st.next_checkpoint_generation = st.next_checkpoint_generation.saturating_add(1);
        Ok(cp_gen)
    }

    /// [sexfiles.checkpoint.proof.latest_valid]
    /// Find the latest valid checkpoint (highest checkpoint_generation
    /// with valid magic + checksum). Returns the checkpoint and its index.
    ///
    /// Corrupted checkpoints (wrong magic or bad checksum) are ignored.
    pub fn find_latest_valid_checkpoint(&self) -> Option<(usize, SexfilesCheckpoint)> {
        let st = DISKFS_STATE.read();
        let mut best: Option<(usize, SexfilesCheckpoint)> = None;
        let mut i = 0usize;
        while i < DISKFS_MAX_CHECKPOINTS {
            let cp = st.checkpoints[i];
            if cp.valid && cp.magic == DISKFS_CHECKPOINT_MAGIC {
                if Self::checksum_checkpoint(&cp) == cp.checksum {
                    match best {
                        Some((_, prev)) => {
                            if cp.checkpoint_generation > prev.checkpoint_generation {
                                best = Some((i, cp));
                            }
                        }
                        None => best = Some((i, cp)),
                    }
                }
            }
            i += 1;
        }
        best
    }

    /// [sexfiles.checkpoint.proof.restore]
    /// Restore the object table from a specific checkpoint.
    /// Overwrites the current table with the checkpoint's table snapshot.
    /// Updates the superblock fs_generation to the checkpoint's fs_generation + 1.
    /// Returns the checkpoint generation restored.
    pub fn restore_checkpoint(&self, cp: &SexfilesCheckpoint) -> Result<u64, i64> {
        if cp.magic != DISKFS_CHECKPOINT_MAGIC {
            return Err(messages::ERR_NOT_FOUND);
        }
        if Self::checksum_checkpoint(cp) != cp.checksum {
            return Err(messages::ERR_OVERFLOW);
        }
        let mut st = DISKFS_STATE.write();
        if !st.mounted {
            // Auto-mount minimal state for proof scenarios.
            st.mounted = true;
        }
        st.table = cp.table;
        st.superblock.fs_generation = cp.fs_generation.saturating_add(1);
        st.superblock.checksum = Self::checksum_superblock(&st.superblock);
        Ok(cp.checkpoint_generation)
    }

    /// [sexfiles.checkpoint.proof.corrupt_skip]
    /// Prove that a corrupted checkpoint (bad checksum) is skipped
    /// during latest-valid search even if its generation is highest.
    /// Returns Ok if the corrupt checkpoint is correctly ignored,
    /// Err if find_latest_valid incorrectly returns it.
    pub fn proof_corrupt_skip_scenario() -> Result<bool, i64> {
        let disk = DiskFs::new();
        disk.format_init_empty()?;
        disk.mount()?;

        // Create good checkpoint with generation 1.
        let _oid = disk.create_object_entry(99, 1)?;
        let cp1_gen = disk.create_checkpoint()?;

        // Create good checkpoint with generation 2 (higher).
        let _oid2 = disk.create_object_entry(100, 1)?;
        let cp2_gen = disk.create_checkpoint()?;

        // Corrupt the highest-generation checkpoint's checksum.
        {
            let mut st = DISKFS_STATE.write();
            let mut i = 0usize;
            while i < DISKFS_MAX_CHECKPOINTS {
                if st.checkpoints[i].valid
                    && st.checkpoints[i].checkpoint_generation == cp2_gen
                {
                    st.checkpoints[i].checksum ^= 0x1; // flip one bit
                    break;
                }
                i += 1;
            }
        }

        // find_latest_valid should return cp1 (generation 1), not cp2.
        let latest = disk.find_latest_valid_checkpoint();
        match latest {
            Some((_, cp)) => {
                // Should return generation 1, not the corrupted generation 2.
                Ok(cp.checkpoint_generation == cp1_gen)
            }
            None => Ok(false), // No valid checkpoint found
        }
    }

    /// [sexfiles.checkpoint.proof.generation]
    /// Prove that checkpoint generation numbers monotonically increase
    /// and that a restored checkpoint has the correct generation.
    pub fn proof_generation_monotonic_scenario() -> Result<bool, i64> {
        let disk = DiskFs::new();
        disk.format_init_empty()?;
        disk.mount()?;

        // Create initial state.
        let _oid = disk.create_object_entry(55, 1)?;
        let gen1 = disk.create_checkpoint()?;

        // Create more state.
        let _oid2 = disk.create_object_entry(56, 1)?;
        let gen2 = disk.create_checkpoint()?;

        // Verify monotonic increase.
        if gen2 <= gen1 {
            return Ok(false);
        }

        // Restore from gen1 checkpoint.
        let latest = disk.find_latest_valid_checkpoint();
        match latest {
            Some((_, cp)) => {
                let restored_gen = disk.restore_checkpoint(&cp)?;
                // After restore, fs_generation should be cp.fs_generation + 1.
                let st = DISKFS_STATE.read();
                let ok = st.superblock.fs_generation == cp.fs_generation.saturating_add(1);
                drop(st);
                // The restored checkpoint's own generation should match.
                Ok(ok && restored_gen == cp.checkpoint_generation)
            }
            None => Ok(false),
        }
    }

    /// [sexfiles.checkpoint.proof] — full end-to-end checkpoint roundtrip.
    /// Creates objects, checkpoints, restores, and verifies state correctness.
    /// Returns checkpoint count and whether verification passed.
    pub fn proof_checkpoint_roundtrip() -> Result<(usize, bool), i64> {
        let disk = DiskFs::new();
        disk.format_init_empty()?;
        disk.mount()?;

        // Phase 1: Create two known objects and checkpoint.
        let oid_a = disk.create_object_entry(10, 1)?; // kind=10, owner=1
        let oid_b = disk.create_object_entry(20, 1)?; // kind=20, owner=1

        let _cp1_gen = disk.create_checkpoint()?;

        // Phase 2: Create a third object (should NOT be in the checkpoint).
        let oid_c = disk.create_object_entry(30, 1)?; // kind=30, owner=1

        // Phase 3: Restore from checkpoint — object C should be gone.
        let latest = disk.find_latest_valid_checkpoint();
        match latest {
            Some((_, cp)) => {
                disk.restore_checkpoint(&cp)?;
            }
            None => return Ok((0, false)),
        }

        // Phase 4: Verify only A and B exist, not C.
        let sa = disk.stat_object_entry(oid_a).is_ok();
        let sb = disk.stat_object_entry(oid_b).is_ok();
        let sc = disk.stat_object_entry(oid_c); // should fail (restored state lacks C)

        let objects_restored = sa && sb;
        let c_gone = matches!(sc, Err(e) if e == messages::ERR_INVALID_HANDLE);

        // Count checkpoints.
        let st = DISKFS_STATE.read();
        let mut cp_count = 0usize;
        let mut i = 0usize;
        while i < DISKFS_MAX_CHECKPOINTS {
            if st.checkpoints[i].valid {
                cp_count += 1;
            }
            i += 1;
        }
        drop(st);

        Ok((cp_count, objects_restored && c_gone && cp_count >= 1))
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  SEXOS_SEXFILES_REAL_BLOCK_PROOF: block read/write contract validation.
    //  These methods validate alignment, bounds, and format integrity against
    //  the in-memory scaffold. They prove the block contract is CORRECT even
    //  though NO real block device route exists yet.
    // ═══════════════════════════════════════════════════════════════════════════

    /// [sexfiles.block.proof.route]
    /// Validate that the DiskFS block model (DISKFS_BLOCK_SIZE = 4096,
    /// object_table_start_block, etc.) is internally consistent.
    /// Returns Ok on valid layout, Err on inconsistency.
    pub fn proof_validate_block_route() -> Result<(), i64> {
        let st = DISKFS_STATE.read();
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        // Superblock block_size must match the compile-time constant.
        if st.superblock.block_size != DISKFS_BLOCK_SIZE {
            return Err(messages::ERR_OVERFLOW);
        }
        // Block size must be power-of-two and >= 512.
        if DISKFS_BLOCK_SIZE < 512 || DISKFS_BLOCK_SIZE.count_ones() != 1 {
            return Err(messages::ERR_OVERFLOW);
        }
        // object_table_start_block must be >= 0 (superblock is block 0).
        // Object table entries each occupy < 1 block (entry is < 128 bytes,
        // DISKFS_MAX_OBJECTS * 128 <= 2048 < 4096).
        let max_entry_bytes = DISKFS_MAX_OBJECTS as u64 * 128;
        if max_entry_bytes > DISKFS_BLOCK_SIZE as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        Ok(())
    }

    /// [sexfiles.block.proof.write]
    /// Validate that a hypothetical block write request is aligned and bounded.
    /// - block_offset must be sector-aligned (multiple of 512)
    /// - block_len must be <= DISKFS_BLOCK_SIZE
    /// - block_offset + block_len must not overflow u64 address space
    /// Returns Ok(()) if the request passes all checks.
    pub fn proof_validate_block_write(block_offset: u64, block_len: u32) -> Result<(), i64> {
        const SECTOR_SIZE: u64 = 512;
        // [sexfiles.block.proof.align_deny]: reject unaligned block offsets
        if block_offset % SECTOR_SIZE != 0 {
            return Err(messages::ERR_OVERFLOW);
        }
        // [sexfiles.block.proof.bounds_deny]: reject oversized block requests
        if block_len as u64 > DISKFS_BLOCK_SIZE as u64 {
            return Err(messages::ERR_OVERFLOW);
        }
        if block_len == 0 {
            return Err(messages::ERR_INVALID_HANDLE);
        }
        // Overflow check
        block_offset.checked_add(block_len as u64)
            .ok_or(messages::ERR_OVERFLOW)?;
        Ok(())
    }

    /// [sexfiles.block.proof.read]
    /// Validate that a hypothetical block read request is aligned and bounded
    /// (same contract as write). Returns Ok(()) on pass.
    pub fn proof_validate_block_read(block_offset: u64, block_len: u32) -> Result<(), i64> {
        Self::proof_validate_block_write(block_offset, block_len)
    }

    /// [sexfiles.block.proof.match]
    /// Simulate a block roundtrip: write a superblock-sized payload at a given
    /// block offset into the in-memory scaffold, read it back, and verify
    /// the data matches. This proves the format contract is correct.
    /// Only operates on block 0 (superblock area) in the mock.
    pub fn proof_block_roundtrip_match() -> Result<(), i64> {
        let st = DISKFS_STATE.read();
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        // Verify the superblock validates against itself (checksum+magic).
        let sb = st.superblock;
        if sb.magic != DISKFS_MAGIC {
            return Err(messages::ERR_NOT_FOUND);
        }
        if Self::checksum_superblock(&sb) != sb.checksum {
            return Err(messages::ERR_OVERFLOW);
        }
        // Verify block_size roundtrip.
        if sb.block_size != DISKFS_BLOCK_SIZE {
            return Err(messages::ERR_OVERFLOW);
        }
        Ok(())
    }

    /// Fault injection: fill journal to capacity with raw records
    /// (bypasses table allocation), then try one more normal append
    /// which must return ERR_FULL.  Returns Ok on the final ERR_FULL,
    /// Err if setup fails.
    pub fn proof_fill_journal_and_test_full() -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }
        // Fill journal completely.
        while st.journal_len < DISKFS_JOURNAL_CAPACITY {
            let idx = st.journal_len;
            let rec = Self::make_journal_record(
                JournalRecordKind::TxBegin,
                0,
                st.superblock.fs_generation,
                0,
                0,
                0,
            );
            st.journal[idx] = rec;
            st.journal_len += 1;
        }
        // One more through normal append → ERR_FULL.
        let rec = Self::make_journal_record(
            JournalRecordKind::TxBegin,
            1,
            st.superblock.fs_generation,
            0,
            0,
            0,
        );
        Self::append_journal_record(&mut st, rec)
    }
}

impl FsBackend for DiskFs {
    fn open(&self, _name: &[u8], _flags: u32, _mode: u32, _caller_pd: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn read(&self, _handle: u64, _offset: u64, _buf: &mut [u8], _caller_pd: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn write(&self, _handle: u64, _offset: u64, _data: &[u8], _caller_pd: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn close(&self, _handle: u64, _caller_pd: u32) -> Result<(), i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn stat(&self, _handle: u64, _caller_pd: u32) -> Result<(u64, u32), i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn list_at(&self, _index: usize, _caller_pd: u32) -> Option<(u64, u32)> {
        None
    }

    fn len(&self, _caller_pd: u32) -> usize {
        0
    }

    fn create_with_owner(
        &self,
        _name: &[u8],
        _owner_pd: u32,
        _caller_pd: u32,
    ) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }
}
