# PHASE2_CHECKPOINT_ROLLBACK_BOOT_RECOVERY_V1

## Result

STOP FIRST.

Phase 2 scheduler wake-queue implementation was rolled back to the Phase 1
checkpoint and Phase 2-only untracked artifacts were moved aside. Build and
visual boot proof did not complete because the canonical build failed before
producing a fresh ISO.

Phase 2 remains suspended.

## Why HEAD Was Not Used

`git restore --source=HEAD kernel/src/scheduler.rs` was not used. HEAD predates
the Phase 1 scheduler boundary and would lose the `ScheduleDecision` /
`NoRunnable` ownership fix. The authoritative rollback source for this mission
was:

`kernel/src/scheduler.rs.bak.wake_queue_v1`

## Checkpoint Validation

Checkpoint comparison:

```text
cmp kernel/src/scheduler.rs.bak.wake_queue_v1 kernel/src/scheduler.rs.bak.no_runnable_v1
result: different
```

Checkpoint SHA-256:

```text
71db38485b65ea8c9cf17caddac165d139fd1527cdd55d28dea5945bf133bd79  kernel/src/scheduler.rs.bak.wake_queue_v1
```

Phase 1 markers in checkpoint:

```text
ScheduleDecision: present
NoRunnable: present
unpark_thread: present
WakeQueue: absent
wake_task: absent
```

## Backup

Pre-rollback backup path:

```text
/tmp/sexos_phase2_before_checkpoint_rollback_20260721-205857
```

Saved files and evidence:

```text
status.txt
all_dirty.patch
files/scheduler.rs.phase2
files/scheduler.rs.phase1_checkpoint
files/kernel_src_interrupts.rs
files/kernel_src_lib.rs
files/kernel_src_graphics_handoff.rs
files/kernel_src_ipc_router.rs
files/kernel_src_xipc_router.rs
files/sexos_build_spec.toml
phase2_scope.patch
phase2_scope.patch.sha256
phase2_untracked/
```

Patch hashes:

```text
760ad87f98eba0e77565099539085004c6f78c9d7afa7e70f7f3aca384db7e43  all_dirty.patch
2b4b8919d763b39e81173fa61eea7a59af27b47bf5a84cb1ce7a7573df29462c  phase2_scope.patch
```

## Restored Files And Hunks

Restored from checkpoint:

```text
kernel/src/scheduler.rs
```

Manual Phase 2 hunk reversals:

```text
kernel/src/lib.rs
  removed crate::scheduler::run_phase2_wake_proof()

kernel/src/interrupts.rs
  route_interrupt: wake_task/WakeDisposition match restored to unpark_thread

kernel/src/ipc/router.rs
  route_signal: wake_task/WakeDisposition match restored to unpark_thread
  send_reply: wake_task/WakeDisposition match restored to unpark_thread

kernel/src/xipc/router.rs
  route_signal: wake_task/WakeDisposition match restored to unpark_thread

kernel/src/graphics/handoff.rs
  ship_to_sexdisplay: wake_task/WakeDisposition match restored to unpark_thread
```

Moved aside, not deleted:

```text
/tmp/sexos_phase2_before_checkpoint_rollback_20260721-205857/phase2_untracked/scripts/scheduler_exact_wake_queue_gate.sh
/tmp/sexos_phase2_before_checkpoint_rollback_20260721-205857/phase2_untracked/docs/handoff/SCHEDULER_EXACT_WAKE_QUEUE_V1.md
```

## Preservation

`git diff --check` passed.

No compiled Phase 2 implementation symbols remain in `kernel/src` for:

```text
WakeQueue
wake_task
WakeDisposition
WakeError
scheduler.wake
WAKE_QUEUE
LIFECYCLE_LOCK
```

Protected dirty-file diffs were compared against the pre-rollback
`all_dirty.patch` and were unchanged:

```text
apps/spindle/src/main.rs
crates/sex-pdx/src/lib.rs
docs/handoff/GATE_0_2_LAST_RUN.md
kernel/src/syscalls/mod.rs
servers/quil/src/main.rs
servers/sexfiles/src/messages.rs
servers/sexfiles/src/vfs.rs
```

`sexos_build_spec.toml` is byte-identical to the scoped pre-rollback backup.

## Phase 1 Preservation

Current source retains Phase 1 markers:

```text
kernel/src/scheduler.rs: ScheduleDecision, NoRunnable
kernel/src/interrupts.rs: ScheduleDecision handling at tick callers
kernel/src/lib.rs: ScheduleDecision handling at debug kick
```

Old wake API is restored:

```text
kernel/src/scheduler.rs: unpark_thread definition
kernel/src/interrupts.rs: route_interrupt uses unpark_thread
kernel/src/ipc/router.rs: route_signal and send_reply use unpark_thread
kernel/src/xipc/router.rs: route_signal uses unpark_thread
kernel/src/graphics/handoff.rs: ship_to_sexdisplay uses unpark_thread
```

ABI hash regeneration was not needed:

```text
expected=1e1fcc5840e6fbe16c31918a94d2d2d7426ceee1a4cc5827b40b3521cab29a60
actual=1e1fcc5840e6fbe16c31918a94d2d2d7426ceee1a4cc5827b40b3521cab29a60
ABI_HASH_MATCH
```

## Build Result

Canonical command attempted:

```text
./scripts/entrypoint_build.sh
```

Hard ABI/PKRU/FSM guard rows passed before the build stage. The optional host
`cargo check -p sex-pdx` warned because cargo looked for
`/home/xirtus_arch/x86_64-sex.json`; this is a host-env warning in the guard.

The build stopped at `build_kernel`:

```text
error: no matching package named `nvme-oxide` found
location searched: crates.io index
required by package `sex-kernel v0.1.0 (/home/xirtus_arch/Projects/Sex/kernel)`
```

`CARGO_NET_OFFLINE=true ./scripts/entrypoint_build.sh` and a direct locked
offline kernel build failed with the same `nvme-oxide` resolution error.

No fresh rollback ISO was produced.

## Visual Boot Result

Not run. The available `sexos-v1.0.0.iso` predates this rollback, so booting it
would not prove the restored checkpoint source. Required visual proof remains
blocked behind the canonical build failure.

## Scheduler Recovery Evidence

Runtime evidence was not captured because no fresh ISO was produced.

Required but not proven in this run:

```text
PD1/sexdisplay receives task.running
PD3/silk-shell receives task.running
first selected task is not discarded
scheduler round-robin continues
desktop visible
sexdisplay ready
silk-shell ready
silkbar ready
clock advances
```

Serial log `/tmp/sexos_phase1_checkpoint_recovery.log` was not generated.

## Gates

Static/source gates run:

```text
./scripts/scheduler_no_runnable_ownership_gate.sh
result: PASS

./scripts/syscall_user_pointer_hardening_gate.sh
result: PASS

./scripts/rsp0_regression_gate.sh
result: PASS
```

Runtime gates not run because they would use a stale ISO after the build failed:

```text
./scripts/gate_0_2.sh
./scripts/master_runtime_gate.sh --skip-build --probe 25
```

## Final State

Rollback state is source-complete and Phase 2 is suspended. Boot recovery is not
proved. Next valid step is to restore canonical Cargo dependency resolution for
`nvme-oxide` without changing rollback scope, rerun `./scripts/entrypoint_build.sh`,
then boot the fresh ISO and capture `/tmp/sexos_phase1_checkpoint_recovery.log`.
