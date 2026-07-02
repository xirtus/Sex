# F2: Collar Authority Map — Spec

**Status:** Approved (Docs/Model only)
**Commit:** *(pending)*
**Build:** N/A (no code changes)

## 1. Purpose

Collar is SexOS's **authority wallet / trust control plane / grant manager**.
It governs what each PD can do, what resources it can access, and under what
conditions. Collar is the **policy owner** for authority decisions. Mesh (F1)
visualizes authority; Collar governs authority.

### Core claims

- **Collar never bypasses PDX/MPK.** All authority decisions flow through
  existing capability slot and MPK/PKEY isolation.
- **Collar never stores raw cross-PD pointers.** All references are capability
  slot IDs, surface IDs, or PD slot numbers — never kernel object handles or
  memory addresses.
- **Collar never lets apps self-grant authority.** All grant provenance must
  trace to a user action or a system boot manifest.
- **Collar never lets the renderer own policy.** sexdisplay renders frames;
  it does not decide what surfaces/apps/PDs are trusted.
- **Collar grants minimum scope by default.** Zero-trust baseline — every grant
  must be explicitly requested and justified.
- **Collar grants must be auditable and revocable.** Every grant produces an
  audit record. Every grant can be revoked by user action or policy change.
- **Secrets must be hardware-isolated from app memory.** Keys, tokens, and
  credentials live in Collar's own MPK-protected domain — never copied into
  app PD memory.
- **User-facing prompts must be unspoofable.** The prompt surface is rendered
  by silk-shell under shell authority, never by the requesting app.

### What Collar is NOT

- **Not a renderer.** Collar never writes to the framebuffer. Prompts and
  grant UI are rendered by silk-shell through existing 0xEC/0xEF/0xEE primitives.
- **Not Mesh.** Mesh (F1) displays the authority graph. Collar owns policy
  decisions. Collar may later emit audit state that Mesh displays, but Collar
  remains the policy owner.
- **Not a filesystem.** Collar does not store app data, documents, or objects.
  Linen (future) manages file/object grants in collaboration with Collar.
- **Not a notification system.** Bell (G1) handles event/notification routing.
  Collar may approve/deny Bell action tokens, but Bell does not make authority
  decisions.

## 2. Authority Object Types

Each authority object represents something that can be granted, denied, or
revoked. Objects are identified by shell-local stable IDs.

| Object Type | Identifier | Description |
|-------------|-----------|-------------|
| `app_identity` | PD slot number + launch manifest hash | Identity of a running PD/app |
| `pdx_route_grant` | (source PD slot, target PD slot) | Permission to invoke a PDX slot on another PD |
| `cap_slot_grant` | (PD slot, cap slot number) | Permission to use a specific capability slot |
| `object_file_grant` | Object/file ID | Permission to read/write/execute an object |
| `surface_display_grant` | surface_id | Permission to create/own/modify a surface |
| `input_focus_grant` | surface_id | Permission to receive keyboard/pointer input focus |
| `device_grant` | Device ID (USB, HID, block) | Permission to access a hardware device |
| `network_grant` | (protocol, address, port) | Permission to communicate over network |
| `secret_key_grant` | Key/token ID | Permission to use a specific secret or cryptographic key |
| `session_unlock_state` | (session ID, level) | Current session trust level (locked/unlocked/verified) |
| `one_shot_action_token` | Token ID | Single-use action authority (e.g., "install this app once") |
| `revocation_record` | (grant ID, timestamp) | Record of a grant being revoked |
| `audit_event` | (grant ID, PD, operation, result) | Log entry for an authority decision |

## 3. Policy Dimensions

Every grant is parameterized along these dimensions:

| Dimension | Values | Description |
|-----------|--------|-------------|
| **who** | PD slot, app identity | The identity requesting or receiving authority |
| **what** | Object/resource ID | The specific resource being accessed |
| **operation** | `read`, `write`, `execute`, `display`, `input`, `network`, `admin` | What operation is allowed |
| **scope** | `session`, `project`, `object`, `one_shot`, `time_bounded(n)` | How long/wide the grant applies |
| **trust_level** | `system`, `local`, `known`, `untrusted`, `remote` | Trust classification of the requester |
| **user_consent** | `granted`, `denied`, `pending_prompt`, `auto_granted`, `auto_denied` | User consent state |
| **expiration** | timestamp or `never` | When the grant automatically expires |
| **privacy_class** | `public`, `internal`, `confidential`, `secret` | Redaction level for audit/log output |
| **proof_marker** | string | Audit proof marker for the grant decision |

## 4. Grant Lifecycle

Every grant follows this state machine:

```
                  ┌─────────────┐
                  │  Requested  │
                  └──────┬──────┘
                         │
                    ┌────▼──────┐
                    │  Prompt   │ ← if user consent required
                    │ Required  │
                    └────┬──────┘
                         │
              ┌──────────┼──────────┐
              │          │          │
        ┌─────▼───┐ ┌───▼────┐ ┌───▼──────┐
        │ Granted │ │ Denied │ │ Expired  │
        └────┬────┘ └────────┘ └────┬─────┘
             │                      │
        ┌────▼─────┐          (terminal)
        │ Revoked  │
        └────┬─────┘
             │
        ┌────▼──────┐
        │  Faulted  │ ← PD crash / revoke-on-fault
        └────┬──────┘
             │
        ┌────▼───────┐
        │  Audited   │ ← terminal after full audit trail
        └────────────┘
```

**Transitions:**
| From | To | Trigger |
|------|----|---------|
| Requested | PromptRequired | Authority not pre-approved, user must consent |
| Requested | Granted | Auto-grant (system trust, manifest pre-approval) |
| Requested | Denied | Policy reject (untrusted, no manifest match) |
| PromptRequired | Granted | User approved |
| PromptRequired | Denied | User denied |
| Granted | Expired | Time-to-live reached |
| Granted | Revoked | User action, policy change, dependency revoked |
| Revoked | Audited | Audit trail finalized |
| Granted | Faulted | PD crashed while holding grant |
| Denied | Audited | Audit trail finalized |
| Expired | Revoked | Cleanup after expiry |

## 5. Invariants

1. **Never bypass PDX/MPK.** All Collar decisions are advisory to the existing
   kernel-enforced capability and MPK/PKEY isolation. Collar cannot directly
   grant or revoke kernel capabilities — it signals intent, and the kernel
   enforces.

2. **No raw cross-PD pointers.** All grant references are slot numbers, surface
   IDs, or other shell-local identifiers. No kernel object handles or memory
   addresses cross PD boundaries.

3. **No app self-grant.** Every grant must trace provenance to a user action,
   boot manifest, or system policy. Apps cannot self-authorize.

4. **Minimum scope by default.** Zero-trust baseline. Every operation requires
   an explicit grant. No implicit inheritance.

5. **Auditable and revocable.** Every grant produces an audit record. Every
   grant can be revoked.

6. **Secrets isolated.** Keys, tokens, and credentials live in Collar's own
   MPK-protected domain. Never copied into app PD memory.

7. **Renderer never owns policy.** sexdisplay and silk-shell render prompts
   and grant UI under shell authority. They never decide what is trusted.

8. **Prompts unspoofable.** Grant prompts are rendered by silk-shell on a
   shell-owned surface, never by the requesting app.

## 6. Relationship to Mesh

| Aspect | Mesh (F1) | Collar (F2) |
|--------|-----------|-------------|
| Role | Visualize authority | Govern authority |
| Output | Graph nodes + edges | Grant records + audit events |
| Policy | Never grants/revokes | Owns all grant decisions |
| Data source | Shell-local state | Grant lifecycle table |
| Rendering | Mesh surface (future) | Prompt surfaces + audit log |
| Dependency | Reads Collar audit state optionally | Independent policy layer |

**Key rule:** Mesh displays what Collar decides. Collar never reads Mesh state
to make policy decisions in F1/F2 — Mesh is diagnostic only.

## 7. Relationship to Bell

Bell (G1) handles event/notification routing. Collar and Bell interact as
follows:

- **Bell events may request action tokens.** An event (new device, install
  request, document open) may carry a request that requires authority.
- **Collar approves/denies action authority.** When Bell routes a request,
  Collar checks policy and returns granted/denied.
- **Bell never becomes grant authority.** Bell is a notification router, not
  a policy engine. It does not self-authorize actions.
- **User prompt flow:** Bell event → Collar policy check → prompt (if needed)
  → Collar grants/denies → result routed back through Bell.

## 8. Relationship to Linen/Quil

| Feature | Linen (future) | Quil (future) |
|---------|---------------|---------------|
| Resource | Objects/files/projects | Code/editor/build/run |
| Grant scope | read, write, project, object | edit, build, run, review, project |
| Collar role | Approve file access | Approve build/execute authority |
| D/E state | Placeholder only (no real authority) | Placeholder only (no real authority) |

In F1/F2, D1/D2/E1/E2 placeholders have no real object authority. Real grant
enforcement waits until Linen and Quil have actual resource models.

## 9. Future Implementation Plan

### F2 (this document)
- ✅ Docs/model definition
- No implementation

### F3 — Bell Event Contract (docs)
- Define Bell event routing, notification model, action tokens

### F4 — Collar Placeholder Surface
- Collar surface through proven Scene/Frame/Tab path (mirror D1/E1)
- Toggle via key binding
- Placeholder fill rect only — no real grant UI
- **Requires STOP FIRST review**

### F5 — Collar Audit Event Ring
- Add audit event ring (similar to A6 tombstone ring)
- Record grant decisions, prompts, revocations
- Shell-local, fixed-size ring buffer
- Emits `[collar.audit.*]` proof markers

### F6 — Mesh Displays Collar Audit State
- Mesh reads Collar audit ring
- Displays grant/revoke edges in authority graph
- Collar remains policy owner
- **Requires STOP FIRST review**

### Real Grant Enforcement (future)
- Kernel capability grant/revoke integration
- MPK domain assignment
- Secret storage
- User prompt system
- **Multiple STOP FIRST reviews required**

## 10. STOP FIRST Triggers

Stop all Collar work and escalate if any of the following are required:

- **Kernel edits** — Collar signals intent; kernel enforces
- **`crates/sex-pdx/` ABI/opcode edits** — Uses existing constants only
- **Grant enforcement changes** — No real enforcement until F4+ STOP review
- **Secret storage implementation** — Requires MPK-isolated domain design review
- **Cryptography implementation** — Requires audit and design review
- **Renderer-owned policy** — sexdisplay/silk-shell never decide trust
- **App-controlled prompts** — All prompts rendered under shell authority
- **Shared-memory/backing-buffer redesign** — Uses existing PDX path
- **Cross-PD raw pointers** — Never stored or transmitted
- **Self-granting authority** — Apps cannot self-authorize, ever
