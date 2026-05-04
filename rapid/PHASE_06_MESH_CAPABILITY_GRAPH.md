# PHASE 06: The Living System — Mesh (Self-Awareness) + Collar (Capability Conscience)

## Revolutionary Vision

Every operating system in history has been **blind**. Linux has `/proc`, `top`, `perf`, eBPF — tools that peer into a system not designed to be seen. Security is bolted on: DAC, MAC, LSMs, namespaces, seccomp — layers of denial on top of a foundation that trusts by default.

**Mesh and Collar together change this at the architectural level.**

- **Mesh is not a monitoring tool.** It is the system's **model of itself** — a living, temporal, causally-connected graph that every server is born into. Servers are not "data sources" that feed Mesh; they are _nodes in the Mesh graph_ from the moment they boot. The graph is not constructed by observation — it is the system's identity.

- **Collar is not a permission manager.** It is the **borrow checker of the operating system** — a runtime reflection of Rust's ownership model applied to OS resources. Capabilities are `Rc<Resource>`, borrowing is `&`/`&mut`, revocation is the compiler telling you your reference is stale — except it happens at runtime, in real time, across protection domains.

Together, they form something no OS has ever had: **an immune system** (Collar) with a **nervous system** (Mesh) that together know what the system is, what it should be doing, what it is actually doing, and what to do when those diverge.

This is not "better observability" or "better security." This is a **new category of operating system capability** — one that is self-aware by construction, not by instrumentation.

### Canonical Definitions

```
MESH_CANON="
Mesh is not Device Manager.
Mesh is not Network Settings.
Mesh is not Activity Monitor.
Mesh is not Little Snitch.

Mesh is the living capability graph of the whole computer.
A real-time map of:
- protection domains
- PDX routes
- capabilities
- devices
- files/objects
- network peers
- Sex nodes
- app surfaces
- trusted/untrusted edges
- broken or denied routes
"
```

```
COLLAR_CANON="
Collar is not a password manager.
Collar is not Keychain.
Collar is not 'permissions settings.'

Collar is the authority wallet.
Stores and governs:
- user identity
- device trust
- app trust
- object grants
- capability grants
- secret keys
- unlock state
- security prompts
- revocation history
- authority relationships
"
```

### What Mesh Is Not (Explicitly)

Normal operating systems keep everything hidden:
- processes are hidden behind abstraction layers
- permissions are buried in settings panels
- devices are in a separate management console
- network is in a different panel
- security is yet another app
- crashes are written to logs nobody reads
- users see apps, not the machine

SexOS with Mesh:

**The computer becomes visible as a graph.**

```
                [sexnet]
                   │
          remote capability edge
                   │
[SexNode-Laptop]──[Mesh]──[sexusb]──[USB Mouse]
       │             │
       │             ├──[sexinput]──OP_HID_EVENT──[silk-shell]
       │             │
       │             ├──[sexdisplay]──surface caps──[Linen]
       │             │
       │             ├──[sexfiles]──object caps──[Quil Project]
       │             │
       │             └──[Collar]──grant policy──[App]
       │
   cluster PDX
```

The revolutionary part: **Mesh does not just show devices. It shows authority flow.**

```
MESH_EDGE_TYPES="
PDX_CALL_ALLOWED
PDX_CALL_DENIED
READ_CAP
WRITE_CAP
DISPLAY_SURFACE_CAP
INPUT_FOCUS_ROUTE
FILE_OBJECT_ROUTE
NETWORK_ROUTE
REMOTE_NODE_ROUTE
DEVICE_ROUTE
FAULTED_ROUTE
REVOKED_ROUTE
"
```

So when something breaks, Mesh can explain why — without grep, without log archaeology:

```
Quil cannot open /projects/kernel
  because:
    Quil has read cap
    but not write cap
    Collar denied write grant
    reason: project is protected system tree

USB mouse moved
  sexusb → sexinput → silk-shell → focused Frame
  all green

App tried to screenshot screen
  App → sexdisplay CAP denied
  reason: no screen-capture authority
  Collar prompt available
```

```
WHY_MESH_WINS="
1. It sees the real OS structure, not fake app abstractions.
2. It visualizes PDX/capability authority directly.
3. It can explain failures without grep/log archaeology.
4. It can show local + networked Sex nodes as one graph.
5. It turns microkernel complexity into user-visible power.
6. It makes security understandable without dumbing it down.
"
```

### Collar: Authority, Not Permissions

Normal OS security is mostly:
- allow/deny popup
- password vault
- keychain
- sudo prompt
- app permissions panel

Collar is different. Every meaningful power in SexOS is a capability.
Collar is where those powers are **named, stored, inspected, granted, revoked, and remembered.**

Visualize Collar as an authority map:

```
User Andreas
  ├── owns Device: Xirtus Laptop
  ├── trusts App: Quil
  │     ├── may read /projects/Sex
  │     ├── may write /projects/Sex/apps/quil
  │     ├── may call sexfiles
  │     └── may NOT call sexnet without prompt
  ├── trusts App: Browser
  │     ├── may call sexnet
  │     ├── may display surfaces
  │     └── may NOT read /projects/Sex
  └── system authority
        ├── sexdisplay sole framebuffer writer
        ├── sexinput sole input producer
        └── silk-shell owns focus/session policy
```

Collar grants are not vague. They are scoped precisely:

```
COLLAR_GRANT_EXAMPLES="
grant Quil read:/projects/Sex
grant Quil write:/projects/Sex/apps/quil
grant Mesh inspect:PD_GRAPH readonly
grant Browser network:https only
grant App display:surface_create
grant App notification:normal
deny App framebuffer:raw
deny App input:global_keylog
"
```

The best part: **revocation is real.** Collar can revoke one file grant, one app's network grant, one session token, one device trust edge, one PDX route, one remote node trust — and Mesh can show the result immediately.

```
Collar = authority control
Mesh   = authority visualization
```

### Collar Security Model

```
COLLAR_SECURITY_MODEL="
- written in strict no_std Rust
- isolated Protection Domain
- secrets never shared as raw pointers
- PDX-only authority requests
- MPK/PKU memory isolation
- bounded capability objects
- revocation log
- user-visible trust graph
- optional hardware-backed sealing later
- optional GPU/TPM/secure-element/remote attestation later
"
```

About security claims: the credible claim is not "impossible to crack." The credible claim is:

```
SECURITY_CLAIM="
Collar is designed so secrets and authority are hardware-isolated,
capability-scoped, auditable, revocable, and minimized by default.
"
```

### Future Collar Vision (V2+)

```
COLLAR_ULTIMATE="
- post-quantum crypto algorithms for future network/node identity
- hardware-backed local device keys where available
- sealed secrets per device/session/app
- visual secure attention path through sexdisplay
- Mesh-visible grant graph
- one-click revoke and replay
- no hidden ambient authority
"
```

This is world-class because SexOS is capability-native from the bottom — not bolting permissions onto UNIX.

### The Trinity

```
TRINITY="
Quil edits the system.
Mesh shows what the system is and how it connects.
Collar controls who/what has authority.
"
```

**Without them:** SexOS is powerful but invisible.

**With them:** SexOS becomes understandable, controllable, and programmable.

### Mesh: The System's Self-Model

```rust
/// Every entity in the system is a GraphNode.
/// Created at boot, registered by the kernel, never garbage-collected.
enum GraphNode {
    Pd { id: u32, name: FixedStr<64>, boot_time: Ticks },
    CapabilitySlot { pd_id: u32, slot: u32, bound_to: Option<u32> },
    PdxRoute { from_pd: u32, to_pd: u32, opcode: u64, last_call: Ticks, call_count: u64 },
    Surface { id: u64, owner_pd: u32, label: FixedStr<32> },
    Device { bus: BusKind, address: u64, class: DeviceClass, driver: Option<u32> },
    Service { name: FixedStr<64>, pd_id: u32, protocol: u64 },
}
```

Key revolutionary properties:

1. **Born-into, not fed-to.** The kernel's PD spawner registers each new PD as a Mesh node automatically. No server code needs to "register" with Mesh. The act of existing in the system creates your node.

2. **Temporal by default.** Every node and edge carries a `since: Ticks` and an optional `until: Ticks`. Dead PDs don't disappear from the graph — they get marked `until: T_death`. You can query: "Show me the system graph at T+30s." This is not an audit log — it's **time-travel for the entire system model**.

3. **Causal edges, not just routing edges.** Mesh tracks not just "A calls B" but "A calls B because C requested X." The graph carries provenance: `Edge { from: Pd(sexinput), to: Pd(silk-shell), reason: "forwarding mouse event from sexusb", via: PdxRoute { opcode: 0x202 } }`. Every edge explains _why_ it exists.

4. **Live queries, zero-copy.** Any server can ask Mesh: "Who holds a capability to my slot 3?" "How many calls has sexdisplay handled in the last 1000 ticks?" "Show me all PDs that have been revoked in the last minute." Mesh answers from its in-memory graph in constant time — no log scanning, no aggregation.

5. **Pattern bounds.** Mesh tracks baseline behavior: "sexdisplay normally processes 55-65 surface updates per second." If the count drops to 0 or spikes to 200, Mesh flags an anomaly edge on the graph. Not AI — simple statistical bounds on a rolling window. The system detects its own degradation before users do.

### Collar: The Runtime Borrow Checker

```rust
/// A capability is a first-class runtime object with ownership semantics.
/// Modeled after Rust's ownership types — not after POSIX's "everything is a file."
struct Capability {
    resource: ResourceId,    // What: slot, surface, path, device
    operations: OpSet,       // Which: read, write, execute, manage
    origin: Origin,          // Where: boot manifest, user grant, parent delegation
    lifetime: Lifetime,      // When: borrow, lease, owned
    audit_level: AuditLevel, // How: silent, log, prompt, require_reason
}
```

Key revolutionary properties:

1. **Capabilities are typed objects, not bits.** A capability is not a flag in a bitmask. It is a `Capability` struct with metadata: what resource, what operations, how long, who granted it, who can revoke it, what audit level. The type system enforces that you can't use a `ReadCapability` as a `WriteCapability`.

2. **Borrow-checker semantics.** `Capability::borrow(source, duration)` creates a derived capability with narrowed scope and automatic return. The original capability is suspended for the borrow duration — just like Rust's `&mut`. If the borrower doesn't return it, Collar detects the lease expiry and revokes the derived capability automatically. **No dangling capabilities.**

3. **Capability decay.** All capabilities weaken over time:
   ```
   Grant:   ReadWrite + forever    (from boot manifest)
   After 1h: → ReadOnly + 24h     (automatic decay)
   After 24h: → ReadOnly + prompt (needs user re-consent)
   After 7d:  → Revoked           (must be re-granted)
   ```
   The user never needs to remember to revoke access. Capabilities erode naturally. Only explicitly renewed grants persist.

4. **Promptless consent.** Collar learns patterns. "App A opens file X at every boot. I've seen this 10 times. Auto-grant with audit." The first time is a prompt; the 10th time is automatic. But every auto-grant is revocable with a single click. Trust is earned, not manual.

5. **Capability provenance (fully auditable).** Every capability carries its origin chain:
   ```
   Capability: Read(/home/user/docs) granted to App A
     Origin: boot manifest → user consent at T+10s → delegated to App B at T+30s → returned at T+35s
   ```
   This is not a separate audit log. **The capability IS the audit trail.** You can't lose the audit because the audit is embedded in the capability itself.

6. **Zero-overhead for trusted paths.** If a capability never transits a trust boundary, it never passes through Collar. Two servers in the same trust domain communicate directly via PDX with no Collar mediation. Collar is a routing layer that can be compiled away. **Zero cryptographic overhead for intra-domain operations.**

### How Mesh and Collar Dance

```
┌─────────────────────────────────────────────────────────────────┐
│                        MESH (Nervous System)                     │
│  Knows: who exists, what they do, how often, what's normal      │
│  Exposes: live graph, temporal queries, pattern bounds          │
│  Says: "sexdisplay's update rate dropped to 0 — it may be stuck" │
└─────────────────────────────────────────────────────────────────┘
                              │ alerts
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      COLLAR (Immune System)                       │
│  Knows: who has what capability, who granted it, when it expires │
│  Exposes: borrow/return, decay, provenance, promptless consent  │
│  Says: "App A's network capability revoked — behavior anomaly"  │
└─────────────────────────────────────────────────────────────────┘
                              │ capability edges
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    QUIL (Human Window)                            │
│  Shows: living graph with color-coded health and authority       │
│  Allows: rewind system state, inspect capability chains          │
│  Enables: revoke any capability, see why it was granted          │
└─────────────────────────────────────────────────────────────────┘
```

**Concrete example of the dance:**

1. App A requests network access.
2. Collar checks: "Has App A asked before? Yes, 50 times. Auto-grant with audit."
3. Collar creates a `Capability { resource: Network, operations: Connect, lifetime: decay(24h), origin: "App A manifest + pattern match" }`.
4. Collar pushes the capability edge to Mesh: `Edge { from: Collar, to: App A, kind: CapabilityGrant, label: "Network access (auto)" }`.
5. Mesh records it in the temporal graph with `since: T+now`.
6. Later, Mesh detects App A's network behavior is anomalous (connection rate 10x normal).
7. Mesh flags an anomaly edge: `Edge { from: Mesh, to: App A, kind: Anomaly, label: "Network rate 10x baseline" }`.
8. Collar sees the anomaly edge, checks its policy: "If anomaly and capability is auto-granted → downgrade to prompt."
9. Collar narrows the capability: `Capability { lifetime: prompt, audit_level: every_access }`.
10. Next time App A connects, the user sees a Quil indicator: "App A is accessing the network. Approve this time? Always? Deny?"
11. The entire chain is visible in Quil's Mesh panel as a color-coded graph path.

**No existing OS does this. No existing OS can do this — because no existing OS was built from the ground up with capability-based security and a system-wide self-model.**

---

## Ownership
- **Mesh** (server/exclusive): temporal graph model, live queries, pattern bounds, anomaly detection
- **Collar** (server/exclusive): capability type system, borrow/decay engine, provenance tracking, promptless consent
- **Quil** (consumer): living graph visualization, capability inspector, time-travel controls
- **Kernel** (birth registration): PD spawn → Mesh node creation — ONE new syscall or PDX opcode

## What Already Exists
- PDX capability slot model is the foundation (slots = capabilities at the lowest level)
- Server dependency map defined in manual (can seed the initial graph)
- PDX slot conventions documented (SLOT_DISPLAY=4, SLOT_SHELL=6, SLOT_SILKBAR=5)
- `sexos_build_spec.toml` has crate manifest (boot order = graph topology)
- No graph model, capability type system, or temporal storage exists yet — greenfield

## Bundle: Mesh

| Revolutionary Feature | Implementation | Effort | Priority |
|----------------------|---------------|--------|----------|
| **Birth registration** | Kernel PD spawn writes to Mesh: `Node { Pd { id, name } }` — no server code | 4h | HIGH — foundation |
| **Temporal graph store** | Ring buffer of `GraphEvent` — each event has `kind, since, until` — no DB, just `[GraphEvent; 4096]` | 6h | HIGH — enables time-travel |
| **PDX route map** | Mesh observes PDX slot bindings — not by probing, but by kernel informing Mesh of each bind/unbind | 4h | HIGH |
| **Causal edge provenance** | Every edge has `reason: FixedStr<128>` — set by the creating server, never by Mesh | 3h | High |
| **Live graph queries** | `OP_MESH_QUERY { filter } → GraphSnapshot` — any server can ask "what is connected to me?" | 4h | High |
| **Pattern bounds engine** | Rolling-window counters per edge type — "normal rate = mean ± 2σ" — flag anomaly when outside bounds | 6h | Medium |
| **Temporal rewind in Quil** | Slider control: drag to T-5min, see the graph as it was. Implemented by replaying events up to that tick. | 4h | Medium |
| **Anomaly edges** | Mesh creates its own edges when it detects bounds violations: `Edge { kind: Anomaly, status: Flagged }` | 2h | Medium |

## Bundle: Collar

| Revolutionary Feature | Implementation | Effort | Priority |
|----------------------|---------------|--------|----------|
| **Capability type system** | `enum CapKind { Read, Write, ReadWrite, Execute, Manage }` with type-state transitions | 4h | HIGH — foundation |
| **Capability object with provenance** | `struct Capability { resource, operations, origin: OriginChain, lifetime, audit }` — 64 bytes fixed-size | 4h | HIGH |
| **Borrow/return engine** | `Capability::borrow(scope, duration)` → derived capability. Original suspended. Returned by lease expiry or explicit `return`. | 6h | HIGH — the core innovation |
| **Capability decay schedule** | Decay rules per origin type. `Origin::UserGrant` decays slower than `Origin::AutoPattern`. Fixed schedule (no config needed). | 3h | High |
| **Promptless consent learner** | Per-app-per-resource counter. Threshold=10. After 10 identical prompts → auto-grant with audit log. | 4h | Medium |
| **Provenance query in Quil** | Click any capability → see its full origin chain: "granted by user at T+10s, borrowed by App B at T+30s, returned at T+35s" | 3h | Medium |
| **Revoke with cascade** | Revoking a capability also revokes all derived capabilities (borrows). Each revocation is a Collar → Mesh edge. | 2h | High |
| **Zero-overhead path detection** | If `source_pd == target_pd` or both in same trust domain → Collar is bypassed. Capability is a local reference. | 4h | Low (optimization) |

## Smallest First Step
**Both begin with one node type and one edge type — the simplest possible graph.**

Mesh: Kernel spawns a PD. Mesh creates `GraphNode::Pd { id: 1, name: "silk-shell", boot_time: T+0 }`. One node. That's it. This proves:
- The kernel can write to Mesh at spawn time
- Mesh stores the node in its temporal ring buffer
- A Quil query returns one node

Collar: Create a capability object. `Capability { resource: Slot(3), operations: Read, origin: BootManifest, lifetime: Forever, audit: Silent }`. Store it. Return it on query. This proves:
- The capability type system compiles
- Capabilities are fixed-size and no_std-safe
- Collar can store and retrieve capabilities

Together: "I boot, a PD spawns, Mesh knows about it, Collar knows what capabilities it has."

## Dependencies
- **Blocking**: Phase 2 (shell model for richer Mesh data — frames, scenes, surfaces)
- **Blocked by**: Nothing for the minimal graph + capability storage. Richer features need Phase 2 (scene model), Phase 3B (USB device nodes)
- **Can parallelize with**: Phase 4 (Linen), Phase 5 (Quil) — Mesh Quil panel is a consumer that can start after the Mesh API stabilizes
- **Kernel integration needed**: ONE opcode or syscall for kernel → Mesh PD spawn notification

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Kernel PD spawn → Mesh notification requires kernel change | High | HIGH | Design as a PDX opcode, not a syscall. Kernel already emits PD spawn events via PDX. If not, silk-shell can register PDs with Mesh on behalf of the kernel (silk-shell knows all PDs). |
| Temporal ring buffer fills up (old events lost) | Medium | Medium | Buffer is circular. Oldest events overwritten. Mesh exposes a watermark: "events before T-X have been rotated." Temporal rewind is valid only as far back as the buffer allows. 4096 events at ~10 events/sec ≈ 6 minutes of history — sufficient for debugging. |
| Pattern bounds engine flags too many false anomalies | Medium | Medium | Start with no automated action — anomalies are visual flags only. Tune bounds empirically before adding auto-response. Anomaly edges are advisory, not authoritative. |
| Capability decay surprises users ("why did my app stop working?") | High | HIGH | Decay is a design choice that must be EXPLICIT. UI shows capability remaining lifetime. Before revocation, Collar sends a warning notification (via Bell, Phase 9). User can renew with one click. Decay is gentle, not sudden. |
| Borrow/return engine is complex to implement correctly | Medium | HIGH | V1: Borrow is a simplified "lease" — fixed duration, no nesting. `Capability::lease(duration)` creates a derived cap. Original is suspended. On expiry OR explicit return, derived is revoked and original is restored. Full borrow-checker semantics (nested borrows, multiple borrowers) deferred to V2. |
| Collar becomes a bottleneck for every capability check | Low | Medium | Trust domain bypass: if A and B are in the same trust domain (same manifest, same origin), Caps are passed directly without Collar mediation. Collar is contacted only for cross-domain capability operations. |
| Quil Mesh panel is too complex (entire OS graph is overwhelming) | Medium | Low | Default view: "Your apps" (filter to user-facing PDs). Expand to "All PDs" on demand. Color-coded by health (green=normal, yellow=anomaly, red=dead/revoked). Graph is explorable but never overwhelming by default. |

## Revolutionary Design Details

### The Mesh Query Language

Not a query language — just PDX opcodes with filter structs:

```rust
// Any server can ask Mesh anything, in constant time.
OP_MESH_QUERY {
    filter: QueryFilter,
    // Examples:
    //   QueryFilter::ByPd { id: 1 }
    //   QueryFilter::ByEdge { kind: EdgeKind::CapabilityGrant }
    //   QueryFilter::ByTime { since: T-1000, until: T_now }
    //   QueryFilter::ByAnomaly { severity: AnomalySeverity::Warning }
}

// Mesh responds with a snapshot:
struct GraphSnapshot {
    node_count: u8,
    edge_count: u8,
    nodes: [GraphNode; 32],       // fixed max per query
    edges: [GraphEdge; 64],       // fixed max per query
    as_of_tick: Ticks,            // when this snapshot was valid
}
```

No serialization, no parsing, no allocation. Fixed-size queries, fixed-size responses. The entire state of the system fits in 2 PDX messages.

### The Collar Capability Protocol

```rust
// Request a capability
OP_COLLAR_REQUEST {
    app_id: u32,
    resource: ResourceId,
    desired_operations: OpSet,
    reason: FixedStr<128>,        // Why? "Need network for firmware update"
}

// Collar responds with:
//   - Granted: Capability { ... lifetime: ... }
//   - Prompt: "User must approve. Reason shown in Quil."
//   - Denied: "Resource not in manifest. Contact developer."
//   - Auto: "Pattern matched. Granted with audit."

// Return a borrowed capability
OP_COLLAR_RETURN { capability_id: u32 }

// Renew a decaying capability
OP_COLLAR_RENEW { capability_id: u32, duration: u32 }
```

### The Mesh + Collar Dance in Quil

A developer opens Quil and sees:

```
┌─────────────────────────────────────────────────────────────────────┐
│  MESH: LIVING SYSTEM GRAPH                    [rewind] ────●──── [now] │
│                                                                     │
│  silk-shell ◄──focus──► sexdisplay    ┌─── anomaly ────► App A     │
│      │                    │            │   (net rate 10x)           │
│      │ cap:slot(3)     cap:slot(4)    ▼                             │
│      ▼                    │          Collar                         │
│  sexinput ◄───hid───────┘            │                              │
│      │                               │ auto-grant w/ audit          │
│      ▼                               ▼                              │
│  sexusb                           App A ──► Network                 │
│                                                                     │
│  [Legend: ● alive  ● dead  ● anomaly  ● revoked]                   │
│  Click any edge → see provenance, rate, last activity               │
└─────────────────────────────────────────────────────────────────────┘
```

Every edge is clickable. Click the `anomaly` edge:
```
Anomaly: App A network rate 10x baseline
  Baseline: 2-5 connections/min (last 24h)
  Current:  47 connections/min
  Since:    T+3420
  Action:   Collar downgraded capability to "prompt" at T+3425
```

Click `auto-grant`:
```
Capability: Network::Connect
  Origin:     App A manifest + pattern match (10/10 identical requests)
  Lifetime:   Decay (24h → read-only → prompt)
  Borrows:    3 active, 47 total
  Revoke:     [CLICK TO REVOKE]
```

This is not a log viewer. This is **time travel for the operating system's consciousness**.

---

## Exit Criteria (Done Checklist)

**Mesh:**
- [ ] Kernel PD spawn → Mesh node created automatically (or silk-shell registers on kernel's behalf)
- [ ] Temporal graph store: 4096 event ring buffer, events have `since`/`until`, queries can filter by time
- [ ] PDX route map: each PDX slot binding recorded as a typed edge with `reason`
- [ ] Live query: `OP_MESH_QUERY` returns `GraphSnapshot` with up to 32 nodes + 64 edges, fixed-size, no alloc
- [ ] Pattern bounds: rolling-window counters per edge type, anomaly flag when outside mean±2σ
- [ ] Quil Mesh panel: living graph with color-coded health, clickable edges with provenance
- [ ] Temporal rewind: slider control shows graph at any point within ring buffer range

**Collar:**
- [ ] Capability type system: `CapKind` enum, `Capability` struct (64 bytes), type-state transitions
- [ ] Borrow/lease engine: `lease(duration)` → derived cap, original suspended, auto-return on expiry
- [ ] Capability decay: auto-decay schedule per origin type, renewal via `OP_COLLAR_RENEW`
- [ ] Promptless consent: per-app-per-resource counter, auto-grant after 10 identical requests
- [ ] Provenance: every capability carries origin chain, queryable in Quil
- [ ] Cascading revoke: revoke → all derived capabilities revoked → Mesh edges updated
- [ ] Zero-overhead bypass: same-domain caps bypass Collar

**Integrated:**
- [ ] Mesh anomaly → Collar auto-response (downgrade capability on anomaly)
- [ ] Quil shows integrated graph with Mesh nodes + Collar capability edges
- [ ] Full capability provenance visible: "granted at T → borrowed at T+30 → returned at T+35 → decayed at T+24h"
- [ ] Build passes. Boot passes. No panic.
- [ ] Only mesh, collar, quil, and sex-pdx changed. No kernel changes beyond one spawn-notification opcode.

## Testing Strategy
- **Mesh temporal graph**: Spawn PD, wait, kill PD. Query at T+0, T+mid, T+now. Verify PD appears at T+0, disappears (marked `until`) at T+death.
- **Pattern bounds**: Send 100 calls via a PDX route in 1 second. Verify baseline establishes. Send 1000 in 1 second. Verify anomaly flagged.
- **Collar borrow/return**: Create capability. Borrow. Verify derived cap exists. Return. Verify original restored. Verify auto-return on lease expiry.
- **Capability decay**: Create cap with 10-tick lifetime. Advance ticks. Verify cap decays to read-only at 5, prompts at 8, revokes at 10.
- **Integration**: Boot all servers. Open Quil Mesh panel. Verify all PDs visible with correct edges. Click any edge. Verify provenance displayed. Rewind to T+0. Verify graph shows boot state.
- **Stress**: 256 PDs, 1024 edges. Verify Mesh handles max load without panic. Verify Quil can render max graph without frame drops.

## Efficiency Opportunity
**The biggest time save is leaning into Rust's type system rather than fighting it.**

The capability model IS Rust's ownership model, projected across domain boundaries:
- `Rc<Capability>` → capability with shared read access
- `Box<Capability>` → exclusive capability (moved, not copied)
- `drop(capability)` → capability revocation

Collar should expose a procedural macro:
```rust
#[require_capability(Network::Connect)]
fn download_update() -> Result<(), Error> {
    // This function is only callable if the caller holds a Network::Connect capability.
    // Collar enforces this at runtime. The macro generates the Collar check.
}
```

This makes capability-aware programming feel like normal Rust — not like "enterprise security middleware."

## Completeness Gain
Observability + Security: **10–20% → 70–85%** (revised upward because this phase delivers a genuinely novel system, not incremental improvements)

## Files Changed
- `servers/mesh/src/main.rs` (new PDX server — temporal graph, live queries, pattern bounds, anomaly detection)
- `servers/collar/src/main.rs` (new PDX server — capability type system, borrow/decay engine, provenance, promptless consent)
- `servers/quil/src/main.rs` (Mesh living graph panel, Collar capability inspector, timeline rewind)
- `servers/silk-shell/src/main.rs` (register PD spawns with Mesh, push frame/scene state as graph nodes)
- `servers/sexdisplay/src/main.rs` (register surfaces with Mesh)
- `crates/sex-pdx/src/lib.rs` (OP_MESH_REGISTER_PD, OP_MESH_QUERY, OP_MESH_PUSH_EDGE, OP_COLLAR_REQUEST, OP_COLLAR_RETURN, OP_COLLAR_RENEW — all in 0xF5..0xFB free range)

## Forbidden
- Kernel enforcement of Collar policy (Collar is advisory in V1 — enforcement remains in PDX slot checks)
- Unbounded graph storage (ring buffer is fixed size)
- Crypto hype / zero-knowledge proofs (capabilities are PDX slot-based, not cryptographic)
- Persistence (graph is in-memory only — temporal range is ring buffer depth)
- Separate audit log (provenance IS the audit trail — embedded in capability objects)
- Garbage collection (dead nodes stay in the temporal graph with `until` set — never removed)

## Next Phase
PHASE_07_APP_LAUNCH_PACKAGE_PATH.md

## Parallel Note
Phase 6 can start as soon as Phase 2 provides the shell model for richer Mesh data. The core Mesh (birth registration, temporal graph, basic queries) and core Collar (capability type system, borrow/lease) are independent of Phase 2 and can start immediately. Mesh and Collar should be developed together — they are two halves of the same living system.

