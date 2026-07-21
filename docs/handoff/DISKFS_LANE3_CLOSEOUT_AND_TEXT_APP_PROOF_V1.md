MISSION: DISKFS_LANE3_CLOSEOUT_AND_TEXT_APP_PROOF_V1

Context (read, don't re-derive): Lane 2 is complete — metadata crash
ordering, corrupt-manifest refusal, allocator rollback, grow/shrink commit
ordering all gated and green (commits 7a32202f, f905780a). Lane 3 slices
1-3 are complete — OP_DISKFS_READ_V2 fixes the sign-bit ambiguity, quil
migrated to it with a staging-buffer fix, spindle migrated, and the
status/len/payload bit layout now lives once in sex_pdx (commits d07edf06,
6b78cf03, 20f6c5b8). Quil already has named-document save (DOC_V1,
4585311d: New -> type -> Save -> name prompt -> OP_DISKFS_CREATE) and
OP_QUIL_OPEN_DISK_DOC to switch identity and reload from an existing
path_id. Spindle already has filldoc/catdoc/truncdoc disk commands.
Everything above landed before/around a same-day scheduler Phase 2
rollback, syscall pointer hardening, and an nvme-oxide dead-dependency
removal (commits c877d455, 74a0b8b0, 13353fea, a06f66cc) that touched nothing
in servers/quil, apps/spindle, or servers/sexfiles — but no full gate sweep
has run since those landed. Do not re-litigate any of the above; verify it,
then extend it.

STOP FIRST per CLAUDE.md: reject any single patch that spans more than two
of {kernel, sexfiles/diskfs, quil, spindle, linen, display} at once. Each
phase below is its own boundary; commit and gate each before starting the
next.

PHASE 0 — REGRESSION SANITY (no code changes)

Run the existing gate suite headless against a fresh build, in order,
stopping and reporting if any fails (don't attempt fixes yet, just
characterize):

  ./scripts/entrypoint_build.sh
  ./scripts/diskfs_v4_growth_gate.sh
  ./scripts/diskfs_v4_crash_injection_gate.sh
  ./scripts/diskfs_v4_manifest_validation_gate.sh
  ./scripts/quil_read_v2_gate.sh
  ./scripts/disk_persistence_gate.sh
  ./scripts/scheduler_no_runnable_ownership_gate.sh
  ./scripts/syscall_user_pointer_hardening_gate.sh

All were last proven green independently; this just proves they're still
green together, post-rollback. Record pass/fail table.

PHASE 1 — LIVE END-TO-END TEXT-APP PROOF (the actual ask: "quil and
spindle taking text input, saving and opening files", proven, not assumed)

Boot the real ISO with a visible window or QMP screendump (genuine visual
proof, not serial-log inference — same standard as the Phase 2 rollback
recovery this session). Drive via QMP input injection or the existing
input-proof harness pattern. Prove, in one continuous session:

  1. Quil: New Buffer -> type distinct content A -> Save -> name it "doc_a"
     -> confirm [quil.doc.create.ok].
  2. Quil: New Buffer -> type distinct content B -> Save -> name it "doc_b".
  3. Quil: open "doc_a" again (via linen or OP_QUIL_OPEN_DISK_DOC directly)
     -> verify buffer content is exactly A, not B and not stale.
  4. Spindle: catdoc doc_a from the terminal -> byte-for-byte matches A.
  5. Spindle: filldoc a third object with content C -> truncdoc to an exact
     shorter length -> catdoc back -> verify exact truncated bytes, not
     torn/padded.
  6. Both surfaces (quil + spindle) visible and responsive in the same
     boot, no faults, no reboot.

Capture serial log + screendump/PNG for each numbered step. This is the
closeout proof for "the file system is finished for quil/spindle" — if any
step fails, that's the real remaining gap, not a hypothetical one.

PHASE 2 — CLOSE DOCUMENTED LANE 3 GAPS (only if Phase 0/1 are clean; each
its own commit)

From docs/handoff/DISKFS_V4_LANE3_READ_V2_V1.md, "explicitly not done":

  a. OP_DISKFS_WRITE_V2 — chunked write opcode. Audited as unnecessary for
     correctness (WRITE isn't ambiguous like READ was) but current
     per-call throughput is ~11B/s at 16-byte round trips; a wider payload
     (same status/len/payload framing sex_pdx already defines for READ_V2)
     would be a real win if quil/spindle saves are the bottleneck users
     feel. Gate with the same before/after byte-exact proof pattern as
     READ_V2's highbit gate.
  b. Interleaved multi-client transfer proof — no existing gate proves two
     concurrent clients (e.g. quil and spindle) reading/writing different
     disk objects at once without cross-talk. Per-caller SELECT state is
     already believed correct; write the gate that actually exercises it.

Skip the explicit length-query opcode (OP_DISKFS_STAT already covers it;
LANE3_READ_V2_V1.md listed it as a maybe, not a real gap).

OUTPUT PER PHASE

A. PASS / FAIL / STOP FIRST
B. Gate table (Phase 0)
C. Per-step proof evidence — log + screendump paths (Phase 1)
D. New gate results (Phase 2, if reached)
E. Files changed, commits made
F. Next decision
