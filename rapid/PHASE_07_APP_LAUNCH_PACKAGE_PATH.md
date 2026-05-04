# PHASE 07: App Launch + Package Path

## Goal
Launcher manifest, app spawn path proof, sexshop package/object metadata, install/list/remove stubs, signed package plan, simple graphical package browser. Make installing and launching apps a proven path — and make every launch visible in the living system graph (Mesh) with proper capability propagation (Collar).

## Revolutionary Angle
**Apps should not "request permissions." They should declare capabilities in their manifest, and Collar grants them automatically — or prompts once and learns.**

When an app launches:
1. Its manifest declares capabilities: `[Network::Connect, Storage::Read("/docs"), Display::Surface(1)]`
2. Collar checks: "Has this app launched before? Yes, 5 times. Previous grants: Network::Connect (auto), Storage::Read (prompt once)."
3. Collar creates capabilities: `Network::Connect` (auto-grant, decay 24h), `Storage::Read` (granted once previously, renew with audit)
4. Collar registers all capabilities as edges in Mesh: `Edge { from: Collar, to: App, kind: CapabilityGrant, label: "Network::Connect (auto)" }`
5. Mesh records the app launch as a temporal event: `Node { App, boot_time: T+now }` with edges to all its capabilities.
6. The user sees the app in Quil's Mesh panel with all its capabilities visible — no digging through settings.

**No separate "permissions screen."** The capability graph IS the permissions screen. Every grant, borrow, decay, and revocation is visible as a typed edge in the living graph.

## Ownership
- **sexshop** (exclusive): package metadata, object store, install/list/remove
- **silk-shell** (integration): launcher surface, app spawn dispatch, surface lifecycle
- **Quil** (consumer): package browser panel
- **Collar/Mesh** (integration): authority grant on install, package node in graph

## What Already Exists
- Surface creation via 0xEC is well-established (used by all app surfaces)
- Focus and surface lifecycle management exist (try_set_focus(), clear_focus_if_dead(), 0xEE destroy)
- SilkBar has launcher button (click toggles launcher panel)
- Launcher panel surface exists (SURFACE_ID_LAUNCHER)
- No app manifest format defined
- No package metadata or sexhop server exists
- No app spawn path (spawn a new PDX server on demand)

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| App manifest format | Metadata: name, icon, capabilities, surface IDs, entry point | 2h | High |
| Launcher app list | SilkBar launcher shows installed apps from manifest | 4h | High |
| App spawn path | Click launcher → spawn PD → surface appears → focus set | 8h | HIGH (core path) |
| sexshop package metadata | Package registry with version, deps, hash, manifest | 6h | Medium |
| Install/list/remove stubs | Basic package lifecycle via sexshop PDX calls | 4h | Medium |
| Signed package plan | Signature verification model (design only — no crypto in V1) | 2h | Low |
| Package browser | Simple graphical browser in Quil showing available packages | 4h | Low |

## Smallest First Step
Create the app manifest format: a fixed-size struct with app name, entry point PD slot, requested surface IDs, and capability list. Store it as a static constant. Then prove you can launch an app by reading the manifest and creating its surface. This is the "hello world" of app launch.

## Dependencies
- **Blocking**: Nominal (can proceed independently)
- **Blocked by**: Phase 6 (Collar) for authority-backed app spawn; Phase 5 (Quil) for package browser
- **Can parallelize with**: Phase 4 (Linen), Phase 5 (Quil text mode), Phase 9 (Network)
- **Key insight**: The app spawn path doesn't need sexshop. Hardcode a manifest for one app, prove the spawn path works, THEN build sexshop for dynamic package management.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| App spawn requires kernel PD creation syscall | High | HIGH | PD creation may need a new syscall. Mitigation: V1 spawns app within an existing PD (silk-shell child) using surface creation only. True PD isolation deferred. |
| App manifest format changes after implementation | Medium | Low | Fixed-size manifest struct. Version field. Reject unknown versions. |
| sexshop needs storage backend | Medium | Medium | Start with in-memory package registry (hardcoded list). Defer persistence to Phase 12. |
| Package signing requires crypto | Low | Low | Design the signature field format. Leave verification as a stub that always passes. Add real verification when crypto library exists. |

## Exit Criteria (Done Checklist)
- [ ] App manifest format defined (fixed-size struct, versioned)
- [ ] At least one app launches from launcher click: spawn → surface → focus
- [ ] `[shell.app.launch]` marker fires on successful launch
- [ ] sexshop returns package list (hardcoded or from sexfiles)
- [ ] Install/list/remove stubs respond to PDX calls (may be no-ops)
- [ ] Package browser in Quil shows available packages
- [ ] Build passes. Boot passes. No panic.

## Testing Strategy
- **Spawn path**: Hardcode a "test app" manifest. Click launcher → verify surface created (grep for 0xEC call), verify focus set.
- **sexshop**: Direct PDX call, verify package list returned, install stub returns success.
- **Integration**: Launch a real app (Quil) from launcher, verify it has correct chrome, focus, and input routing.

## Efficiency Opportunity
**App launch is the single most important path for daily-driver readiness.** Prioritize the spawn path over sexhop features. A working "launch hardcoded app → surface → focus → input" cycle is worth more than a full package manager that can't launch anything.

**Collar integration should be MINIMAL in V1.** App launch can work without Collar authority grants. Add "request permission" as a no-op stub that always grants. Real authority gating is Phase 6's full Collar implementation.

## Completeness Gain
Package/apps: **10–20% → 45–60%** (with hardcoded app + sexshop stubs). **10–20% → 25–30%** (waiting for complete sexhop). Recommendation: ship the spawn path first.

## Files Changed
- `servers/sexshop/src/main.rs` (new PDX server or extend existing — package metadata, install/list/remove)
- `servers/silk-shell/src/main.rs` (launcher manifest parsing, app spawn dispatch)
- `crates/sex-pdx/src/lib.rs` (OP_APP_LAUNCH, OP_SEXSHOP_LIST, OP_SEXSHOP_INSTALL, OP_SEXSHOP_REMOVE opcodes)
- `servers/quil/src/main.rs` (package browser panel)

## Forbidden
- Kernel spawn path rewrite (use existing PDX surface creation pattern)
- App store monolith (sexshop is a metadata/object store, not a storefront)
- Crypto implementation (design signature field, defer verification)
- Broad refactor

## Next Phase
PHASE_08_NETWORK_DEVICE_REALITY.md
