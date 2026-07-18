# STUB_SERVER_KILL_LIST_V1

Audit of 5 orphaned/stub servers. Goal: rank by real OS value vs implementation
risk, scope smallest useful V1 each, recommend what (if anything) Fable should
build first. No code changed this pass — audit + doc only.

## Method

Checked, for each: workspace membership (`Cargo.toml` root `members = [...]`),
build-spec inclusion (`sexos_build_spec.toml`), whether `cargo check` even
finds the package, whether referenced `sex_pdx` types/variants exist in
`crates/sex-pdx/src/lib.rs` today, and whether any live kernel/server code
actually calls into the stub's capability slot.

## A. Ranked list

| Rank | Server | In workspace? | Compiles? | Live caller exists? | Verdict |
|------|--------|---------------|-----------|----------------------|---------|
| 1 | **sext** (demand pager) | No | N/A (not in workspace) | **Yes** — `kernel/src/ipc/pagefault.rs::forward_page_fault`, called from every ring-3 `#PF` in `kernel/src/interrupts.rs:581` | Implement V1 |
| 2 | **sex-ld** (dynamic linker) | No | N/A | No | Design-only V1, no urgency |
| 3 | **sexstore** legacy stub (**sexstore-gui**) | No | N/A | No (zero refs anywhere) | Delete, don't implement |
| 4 | **sexgemini** (native toolchain) | No | N/A | No | Do not attempt — needs own design doc, too big for "V1" |
| 5 | **tuxedo** (DDE broker) | No | N/A | No — zero references outside its own crate | Freeze/delete candidate |

`sexstore` itself (the real KV/object store, 1038 lines) is **not a stub** —
clean `cargo check`, zero `TODO`/`stub`/`mock` markers, wired into
`sexos_build_spec.toml` and the ISO. The "legacy stub" in this family is its
unused GUI shell, `servers/sexstore-gui` (24 lines, `loop {}`, never in
workspace, zero external references).

## B. Findings per stub

### 1. sext — demand pager (HIGHEST VALUE)

**Reality:** not a workspace member, so `cargo check -p sext` can't even
resolve it. Its `_start()` calls `Message::from_u64(req.arg0)` and matches
`MessageType::PageFault` — neither exists in `crates/sex-pdx/src/lib.rs`
today (real `sex_pdx::MessageType` only has `Ping`, `Yield`, `HIDEvent`). File
carries its own comment: `DO NOT ENABLE: requires sext protocol redesign`.

**But the kernel already expects it live.** `kernel/src/interrupts.rs:581`
calls `crate::ipc::pagefault::forward_page_fault(fault_addr, error_code,
task_id)` on every ring-3 page fault (kernel-mode faults take a separate
`KERNEL PAGE FAULT HALT` path, untouched by this). That function does:

```rust
// kernel/src/ipc/pagefault.rs
let msg = MessageType::PageFault { fault_addr, error_code, pd_id, lent_cap: 0 };
safe_pdx_call(2, 0, &msg as *const _ as u64, 0, 0)?;
```

`safe_pdx_call(cap_id=2, opcode=0, ...)` looks up **capability slot 2 in the
faulting PD's own cap table** (not a global sext registry — every PD needs a
slot-2 grant pointing at sext's domain). If that resolves
(`CapabilityData::Domain`), it enqueues
`MessageType::IpcCall { func_id: opcode, arg0, arg1, arg2, caller_pd }` into
sext's `message_ring`. If it does **not** resolve (today's reality — no PD is
ever granted slot 2, sext is never spawned), `forward_page_fault` returns
`Err`, and the fallback in `interrupts.rs:583-585` sends `Signal(11)` (SIGSEGV)
straight to the faulting domain.

**Net effect today:** every real ring-3 page fault in any user PD is fatal —
no lazy mapping, no COW, no swap-in, nothing. It just dies. This is the only
stub on this list with a live, wired, currently-firing call site.

**A real wire-format landmine, found this pass, that must not be re-derived
the hard way:** `kernel/src/syscalls/mod.rs:155` (syscall 28,
`SYSCALL_PDX_LISTEN`) unpacks a dequeued `MessageType::IpcCall` as
`(type_id, caller_pd, arg0, arg1, arg2) = (func_id, caller_pd, arg0, arg1, arg2)`.
Since `forward_page_fault` hardcodes `opcode = 0`, `func_id` is `0`, so
`type_id` comes back as `0` to sext's `pdx_listen_raw()`. But
`pdx_listen_raw` itself treats `type_id == 0` as **"ring empty, yield and
retry"** (`crates/sex-pdx/src/lib.rs:245`). **A page-fault forward with
opcode 0 is indistinguishable from no message at all** — sext would spin
forever and never see it, using the standard wrapper. This is a genuine bug
in the existing kernel-side call, not a new one to introduce.

**STOP FIRST condition:** fixing this needs a one-line kernel change
(`kernel/src/ipc/pagefault.rs`: pass a nonzero opcode, e.g. `0xF0`, instead of
`0`) plus a capability grant (`kernel/src/init.rs`: grant slot 2 → sext domain
for every spawned PD, or at minimum the ones expected to fault). Both are
kernel-side and cross the anti-scope-creep line (kernel + a new server in one
patch) — **do not fold into the same patch as sext's own userspace logic.**
Get an explicit go-ahead before touching `pagefault.rs`/`init.rs`.

**Minimal V1 (userspace side only, `servers/sext`):**
- *Protocol:* on `pdx_listen_raw(0)`, treat `type_id == 0xF0` (post-fix) as a
  page-fault forward. `arg0` is a `u64` that is a **pointer into the kernel's
  own stack frame** holding a `crate::ipc::messages::MessageType::PageFault`
  value — confirm PKU/mapping lets sext's domain actually dereference it
  before assuming this works; if not, the real fix is changing
  `forward_page_fault` to pass `fault_addr`/`error_code`/`pd_id` as
  `arg0`/`arg1`/`arg2` directly (by value, no pointer) — simpler and safer,
  also a kernel-side change, same STOP-FIRST bucket above.
- *State machine:* stateless request/reply. Receive fault → decide
  allocate-a-zero-frame-and-map vs deny → `pdx_reply(caller_pd, status)`.
  V1 scope: always allocate one zeroed frame and map it (no real demand-paging
  policy, no swap, no COW) — this alone converts "every fault is fatal" into
  "first-touch faults resolve," which is the actual value-add.
- *Proof markers:* `[sext.fault.recv] pd=<id> addr=<hex> err=<hex>`,
  `[sext.fault.resolve] pd=<id> ok=1`, `[sext.fault.deny] pd=<id> reason=<str>`.
- *Negative tests:* fault on an address already mapped (must deny, not
  double-map); fault from a PD with no slot-2 grant (must hit the existing
  `Signal(11)` fallback, unchanged); repeated fault storm on same addr (must
  not leak frames — budget/counter it).
- *Files allowed:* `servers/sext/src/main.rs`, its own `Cargo.toml`
  (`workspace.members` addition needs the kernel-adjacent go-ahead above, since
  it changes what the whole workspace builds).
- *STOP FIRST:* any edit to `kernel/src/ipc/pagefault.rs`,
  `kernel/src/interrupts.rs`, or `kernel/src/init.rs` capability grants.

### 2. sex-ld — dynamic linker

**Reality:** not a workspace member. References `sex_pdx::LdProtocol` and
`sex_pdx::StoreProtocol` — **neither type exists anywhere in
`crates/sex-pdx/src`** (confirmed by full-crate grep). `servers/sexshop`
*also* imports `StoreProtocol` from `sex_pdx` in `src/pdx.rs`/`src/trampoline.rs`,
but those two files aren't reachable from `sexshop`'s actual `main.rs` (no
`mod pdx;`/`mod trampoline;` wiring found) — they're dead files sitting
inside a live crate. So `StoreProtocol`/`LdProtocol` are fully fictional today,
not just missing from one file.

**Live caller:** none. Nothing spawns sex-ld, nothing grants it a slot,
nothing calls into it.

**Minimal V1, if pursued:** would need a fresh protocol designed against the
*current* `sex_pdx` primitives (`pdx_listen_raw`/`pdx_reply`/`PdxMessage`),
not the fictional `LdProtocol`/`StoreProtocol`. Scope: `ResolveObject` (name →
mock hash is fine for V1, real hash lookup is sexshop's job later),
`MapLibrary` (hash → PFN, requires a *real* sexshop KV call — sexshop's own
KV opcodes `OP_KV_GET`/`OP_KV_PUT`/`OP_KV_DEL` at `0xB0`/`0xB1`/`0xB2` in
`servers/sexstore/src/main.rs` are the only proven-live store protocol on
this system today; reuse those, don't invent a new `StoreProtocol`),
`GetEntry`, `Stats`.
- *Proof markers:* `[sex-ld.resolve] name=<str> ok=<0|1>`,
  `[sex-ld.map] hash=<hex> pfn=<hex>`, `[sex-ld.entry] hash=<hex> addr=<hex>`.
- *Negative tests:* resolve unknown name (must fail cleanly, not mock a
  hash); map a hash sexstore doesn't have (must propagate the KV-miss, not
  silently return PFN 0).
- *Files allowed:* `servers/sex-ld/src/*.rs`, its `Cargo.toml`.
- *STOP FIRST:* anything touching `servers/sexstore` itself, or any new
  kernel capability slot for sex-ld.
- **Recommendation:** design-only for now. No dynamic-linking consumer exists
  yet (nothing in the OS loads a shared object at runtime) — building this
  before something needs it is speculative work against zero real demand.
  Lower priority than sext.

### 3. sexstore-gui — legacy stub (deprecation only)

24 lines, `_start() { loop {} }`, not in workspace, zero references anywhere
in the tree (`grep -rl sexstore-gui` outside its own dir returns nothing).
Real `sexstore` (the KV/object store server) works standalone and headless —
nothing needs this GUI shell to function.

**Recommendation:** delete `servers/sexstore-gui/` outright, or move to
`archive/` if the team wants to keep the placeholder name reserved. Not a
V1-implement candidate — there's no spec, no caller, no design intent
recoverable from the file itself beyond the empty loop.

### 4. sexgemini — native toolchain server (do not attempt as "V1")

**Reality:** not a workspace member. References `MessageType::CompileRequest`
and `MessageType::Notification` — neither exists in real `sex_pdx`. No
capability slot grant anywhere in `kernel/src/init.rs`. Zero live callers.

**Why this doesn't get a minimal V1:** "native toolchain server" implies
invoking something rustc/cc-equivalent *inside* a `no_std` PD — that's not a
small protocol-plumbing job like the others, it's a from-scratch design
question (what compiler backend runs in ring 3 with no OS underneath it?
cross-compile artifacts staged via sexstore? a full second toolchain ported
to the microkernel?). Scoping a "smallest useful V1" here would either be
fake-small (mock a compile that does nothing real, which is exactly the
current stub's failure mode) or actually enormous. Recommend a dedicated
design doc before any implementation prompt gets written — this one doesn't
belong in a minimal-V1 kill-list.

### 5. tuxedo — DDE (device translation) broker (freeze/delete candidate)

7 + 21 lines total. Comment: `Phase 19: Hardware Translation Logic will
reside here`. Not in workspace, not referenced by any other file in the
codebase (checked `grep -rl tuxedo`), no doc found describing its intended
protocol despite a search of `docs/`. No USB/display/input driver in this
codebase currently calls out to a broker layer — `sexusb`, `sexdisplay`, and
`sexinput` all talk to hardware/each other directly.

**Recommendation:** lowest priority. Nothing in the current architecture
creates demand for a translation-broker indirection layer. Freeze as-is or
delete; do not write an implementation prompt for this until some concrete
driver need for cross-domain hardware translation actually appears.

## C. Recommendation: what Fable should build first

**sext, V1 scope only (userspace side).** It's the only stub with a real,
currently-firing kernel call site waiting on it, and its absence has a
concrete, describable cost today (every ring-3 page fault is fatal). Its V1
is small: one message type, always-allocate-a-zero-frame policy, no
paging/swap policy needed yet. The kernel-side prerequisite fixes (nonzero
opcode, slot-2 capability grants) are real but tiny (one literal, one grant
block mirroring the existing Quil/Linen/Spindle pattern in `init.rs`) —
flagged as their own STOP-FIRST decision, not bundled into Fable's patch.

Everything else on this list is either speculative-with-zero-callers
(sex-ld, sexgemini, tuxedo) or a straight deletion (sexstore-gui).

## D. Implementation prompts (Codex-ready)

### D1. sext V1 (recommended first)

```
Implement servers/sext V1: the SexOS demand-pager PD.

Scope: servers/sext/src/main.rs and servers/sext/Cargo.toml ONLY. Do not
touch kernel/, servers/sexstore, or any other server. If you find you need
a kernel-side change (capability grant, opcode fix), STOP and report it
instead of making it.

Current state: servers/sext/src/main.rs is a dead stub referencing
Message::from_u64 and MessageType::PageFault, neither of which exist in
crates/sex-pdx/src/lib.rs (real MessageType there is only Ping/Yield/
HIDEvent). The crate is not in the root Cargo.toml workspace members list.

Task:
1. Read crates/sex-pdx/src/lib.rs in full — understand PdxMessage,
   pdx_listen_raw, pdx_reply, MessageType as they actually exist today.
2. Read kernel/src/ipc/pagefault.rs and kernel/src/syscalls/mod.rs lines
   ~150-200 (SYSCALL_PDX_LISTEN) to understand exactly what a page-fault
   forward looks like on the wire once it reaches sext's listen loop —
   note the type_id==0 ambiguity documented in
   docs/handoff/STUB_SERVER_KILL_LIST_V1.md section B.1 before assuming
   anything about the opcode.
3. Rewrite servers/sext/src/main.rs: listen loop using real pdx_listen_raw,
   handle the page-fault-forward message shape, always resolve by
   allocating one zeroed frame and mapping it (no swap/COW policy needed),
   reply via pdx_reply with a status code.
4. Add proof markers: [sext.fault.recv] pd=<id> addr=<hex> err=<hex>,
   [sext.fault.resolve] pd=<id> ok=1, [sext.fault.deny] pd=<id> reason=<str>.
5. Do NOT add servers/sext to the root workspace members list — that's a
   build-wide change outside this task's scope. Report that it's needed as
   a follow-up STOP-FIRST decision instead.

Report back: what you changed, what you deliberately did NOT touch (the
kernel-side opcode/capability prerequisites), and any wire-format surprises
you hit that aren't already documented in the handoff doc above.
```

### D2. sex-ld V1 (only if user explicitly asks — no live caller yet)

```
Design (do not necessarily implement without confirmation) servers/sex-ld
V1: SexOS dynamic linker.

Scope: servers/sex-ld/src/*.rs and its Cargo.toml ONLY. Do not touch
servers/sexstore or any kernel file. If sexstore's protocol needs a new
opcode, STOP and report it rather than adding one.

Current state: references sex_pdx::LdProtocol and sex_pdx::StoreProtocol,
neither of which exists anywhere in crates/sex-pdx/src — fully fictional
API. Not in the workspace. Zero live callers anywhere in the codebase (no
kernel capability grant, nothing spawns it).

Task:
1. Read servers/sexstore/src/main.rs — this is the ONLY real, live,
   compiling store server in the codebase. Its opcodes are OP_KV_GET
   (0xB0), OP_KV_PUT (0xB1), OP_KV_DEL (0xB2). There is no ObjectGet/
   ObjectPut hash-addressed API today — only flat KV. Confirm this before
   designing sex-ld's MapLibrary call around a hash-addressed store that
   doesn't exist yet.
2. Design a minimal ResolveObject/MapLibrary/GetEntry/Stats protocol using
   real pdx_listen_raw/pdx_reply against sexstore's actual KV opcodes (key
   = object name or hash, value = PFN or entry point, whatever is simplest
   to represent as a KV pair).
3. Add proof markers: [sex-ld.resolve] name=<str> ok=<0|1>, [sex-ld.map]
   hash=<hex> pfn=<hex>, [sex-ld.entry] hash=<hex> addr=<hex>.
4. Negative tests: resolve of an unknown name must fail cleanly (no mock
   hash fallback); map of a hash sexstore doesn't have must propagate the
   miss, not return a fake PFN.
5. Do not add to workspace members without confirming — report as a
   follow-up decision.

This has no current consumer in the OS (nothing loads a shared object at
runtime yet) — confirm with the user that this is still wanted before
spending real implementation time, since sext (see the other prompt) has a
live caller and this one doesn't.
```

## E. What this pass did NOT do

- No code changes to any of the 5 stubs, `kernel/`, or workspace membership.
- No kernel-side opcode/capability fixes applied (flagged as STOP-FIRST,
  not authorized by this audit).
- `sexstore-gui` not deleted — flagged as a recommendation, deletion itself
  needs explicit confirmation since it's a destructive, if low-risk, action.
