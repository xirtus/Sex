# SEXAUDIO_HARP_PHASE_PLAN_V1

**Status:** Plan only. No implementation.

**Scope warning:** This document covers SexAudio core service and Harp user app design. No kernel audio ABI edits. No PDX ABI edits. No sexdisplay audio path. No POSIX audio assumptions. No raw shared-memory audio buffers without explicit transport gate.

**Audio timing constraint:** SexOS has no std timers, sleep, threads, or POSIX clock APIs. Audio sample timing must use existing deterministic sources: device-interrupt-driven sample clocks (USB isochronous, future HD-Audio IRQ) or shell/input tick counters for non-realtime timing (duck/release envelopes, recording duration). **STOP FIRST** if audio timing design requires new kernel timer ABI, std thread sleep, or POSIX clock API.

**Timing warning:** Audio sample scheduling is NOT real-time guaranteed until an approved deterministic audio timing/device interrupt model exists. V1 proof scenarios may log sample timing but must not assume guaranteed scheduling latency.

---

## 1. Mission

MISSION: SEXAUDIO_HARP_PHASE_PLAN_V1 — Design SexAudio core service and Harp user app/control surface for capability-scoped playback, routing, capture, recording, virtual devices, Bell sounds, accessibility narration hooks, and media playback. Docs/plan only. No implementation.

---

## 2. Why This Missing Phase Exists

Audio was intentionally excluded from the rapid 12-prompt plan and the separate tracks because:

- Audio requires its own capability model (AudioCapability, AudioPrivacyClass) that does not yet exist in Collar.
- Audio requires a routing graph (AudioGraph) that is independent from the compositor/display pipeline.
- Audio requires hardware discovery (sexusb/sexpci) that is not yet mature.
- Audio requires a buffer transport design that cannot assume raw shared memory cross-PD.
- Audio requires separate proof/monitoring from visual desktop events.
- Rushing audio would create an ALSA/PulseAudio/PipeWire clone without capability scoping.

This phase plan is the foundation for SexAudio and Harp. It must be designed independently from compositor, input, accessibility, and storage tracks.

---

## 3. Dependency Gates

1. Collar capability model must support AudioCapability types (output, self-capture, cross-capture, system-mix, microphone, virtual-device) before SexAudio enforces capture/recording policy.
2. PDX IPC must support audio buffer transport (either copy-based or grant-based) — STOP FIRST if raw shared memory is proposed without buffer transport gate.
3. USB audio device discovery (sexusb or future sexpci) must exist before hardware device routes appear in AudioGraph.
4. sexfiles/sexstore must support deterministic file storage before recording session output can be persisted.
5. Bell must be able to emit audio intents (not raw samples) before Bell sounds are mixed by SexAudio.
6. D_ACCESSIBILITY_STACK must emit narration intents before SexAudio routes narration to output.
7. No audio implementation before SA7 audio buffer transport gate is designed and approved.

---

## 4. SexAudio PD Model

SexAudio is a **protection domain (PD)** with its own PDX slot and capability table. Communicates with audio clients via PDX IPC messages. No raw shared memory. No direct hardware MMIO from audio clients.

SexAudio processes audio requests sequentially through its PDX message loop. No concurrent graph mutation. V1 does not support real-time audio threads — all audio operations are synchronous message-handled events. Sample clock timing is device-interrupt-driven (USB isochronous, future HD-Audio IRQ).

---

## 5. SexAudio vs Harp Ownership Split
- Audio clients registration and lifecycle
- Audio capabilities validation (output, capture, recording, virtual devices)
- Audio graph construction, validation, and enforcement
- Mixer: gain, mute, duck, drop, queue, priority scheduling
- Route creation and teardown
- Virtual sinks/sources lifecycle
- Capture sessions enforcement
- Recording sessions enforcement and tombstone
- Priority/duck/drop policy enforcement
- AudioProofEvent production
- Device connection/disconnection handling
- Buffer transport (copy or grant — decided in SA7 gate)

### Harp owns (user-facing control surface):
- Audio graph UI display (what routes exist, which apps are connected)
- Route builder UI (drag source to sink with capability validation prompt)
- Capture/record controls (start/stop recording, choose source)
- Virtual device creation UI (request Collar grant, create virtual sink/source)
- Per-app volume/mute sliders
- Meter display per channel
- Recording session management UI (view active sessions, stop, save)
- Presets/templates (saved route configurations)
- Explaining what is being captured and why (user-visible capture warnings)
- Sends commands to SexAudio after Collar approval — never bypasses enforcement

### Neither owns:
- Hardware bus enumeration (sexusb/sexpci)
- Framebuffer drawing (sexdisplay)
- Grants/secrets/trust (Collar)
- Global graph visualization policy (Mesh — visualizes only)
- Quick controls/capture indicator (SilkBar — shell-owned panel surface)
- Notification/event sound choice (Bell — emits intent, SexAudio routes)
- Proof/debug/dev console (Quil — inspects proof logs)
- Documents/projects/audio files (Linen/sexfiles — stores recordings)

---

## 6. Innovation Goal

SexOS audio should be inspectable sound authority: every stream, sound, tap, virtual device, recording, and route has an owner, capability, privacy class, priority, and proof trail. Harp makes this graph beautiful and controllable without letting apps secretly capture, route, or record each other.

No ALSA/PulseAudio/PipeWire/CoreAudio clone. No app-direct-device MMIO. No hidden recording. No capture without visible proof.

---

## 7. Audio Object Model

- **AudioDevice:** a physical or virtual audio endpoint (output speaker, input mic, virtual sink/source). Has device_id, name, type (PhysicalOutput, PhysicalInput, VirtualSink, VirtualSource), state (Available, Unavailable, Tombstoned).
- **AudioRoute:** a capability-checked edge from one AudioNode to another in the AudioGraph. Represents a directed audio flow.
- **AudioClient:** an actor (PD) that requests audio service. Has client_id, app_identity, owned_capabilities (set of AudioCapability).
- **AudioCapability:** a permission to perform an audio action. Kinds: Output, SelfCapture, CrossCapture, SystemMixCapture, MicrophoneCapture, VirtualDeviceCreate, Recording.
- **AudioIntent:** a request to play, capture, route, or record audio. Contains intent_kind, client_id, target_node, priority, privacy_class.
- **AudioStream:** a live audio stream between two AudioNodes. Has stream_id, source_node, sink_node, client_id, privacy_class, state (Active, Suspended, Tombstoned).
- **AudioBufferRef:** an opaque handle (u64) for audio sample data. **V1 metadata/proof-only:** MUST NOT imply valid cross-PD memory access. NOT a raw pointer, NOT a memory address. Transport mechanism (copy vs grant vs ring) designed in SA7 gate. No buffer transport without SA7 approval.
- **AudioGraph:** the complete directed graph of all audio routes, devices, clients, streams, taps, and virtual nodes. Owned and enforced by SexAudio.
- **AudioNode:** a vertex in the AudioGraph. Can be a Device, Client, MixerChannel, Tap, VirtualSink, VirtualSource, CaptureSession, Recorder.
- **AudioEdge:** a directed connection between two AudioNodes. Contains route_id, capability_checked, state.
- **MixerChannel:** a named channel in the SexAudio mixer. Has channel_id, gain, mute, duck_targets, priority_level, current_streams.
- **MixPolicy:** deterministic rules for gain, mute, duck, drop, queue, priority scheduling across MixerChannels.
- **AudioPriority:** relative importance of an audio stream (Critical: narration/alarms, High: media playback, Medium: notification sounds, Low: decorative/ambient, Background: non-critical).
- **AudioPrivacyClass:** sensitivity level of audio content (Public: notification sounds, Media: app music/video, Narration: accessibility output, Communication: voice/call, Private: user recording, Secure: sensitive/system audio).
- **AudioTap:** a capability-checked graph edge that copies audio from one AudioNode to another for capture/inspection. Has tap_id, source_node, sink_node, client_id, privacy_class.
- **AudioRecorder:** a recording session manager. Has recorder_id, owner_client, source_route, privacy_class, output_file, state (Idle, Recording, Paused, Tombstoned).
- **VirtualAudioDevice:** a device node with no physical hardware. Created on request with Collar approval.
- **VirtualSink:** a virtual device that accepts audio output from apps. Other approved apps/recorders can consume its VirtualSource.
- **VirtualSource:** the output side of a VirtualSink — allows other apps to capture what was sent to the virtual sink.
- **LoopbackRoute:** a route from a mixer bus or device output back into the mixer as a capture source. Requires SystemMixCapture capability.
- **CaptureSession:** an active audio capture operation. Has session_id, capturing_client, source_route, privacy_class, state (Active, Tombstoned).
- **RecordingSession:** an active audio recording operation. Has session_id, recording_client, source_route, privacy_class, output_file, state (Active, Paused, Tombstoned).
- **RecordingPolicy:** rules governing recording: who can record what, what privacy classes can be recorded together, what happens on device disappearance.
- **HarpRoutePatch:** a user-defined route configuration in Harp. Contains source, sink, gain, effects chain (V1: none). Sent to SexAudio as a route creation request.
- **HarpPreset:** a saved route configuration in Harp. Contains AudioGraph subset description, per-channel settings. Restorable.
- **AudioProofEvent:** logged event for any audio operation. Contains event_type, client_id, target_id, capability_used, privacy_class, result (allowed/denied/tombstoned).
- **RouteProofEvent:** logged event for route creation, change, or teardown. Contains route_id, source, sink, client_id, capability_checked, result.

---

## 8. Audio Graph/Routing Model

```
AudioGraph structure (V1):
- MixerChannel[0..N]  (fixed N in V1, configurable in future)
  Each MixerChannel has:
  - gain (u8: 0=mute, 255=max gain, linear or log scale in mixer implementation)
  - mute (bool)
  - duck_targets: list of channel IDs that duck this channel
  - priority_level: AudioPriority
  - attached streams: list of AudioStream

Routing rules:
- Client requests route: client → MixerChannel → device (output)
  or device → MixerChannel → client (capture input)
- Each route is capability-checked at creation time
- Routes are directed edges in AudioGraph
- Routes can be temporary (stream active) or persistent (virtual device)
- Route teardown: device disappears → route tombstones → streams suspended → client notified
- No app-to-app direct route in V1 unless both apps have explicit CrossCapture capability
- Loopback routes (mixer bus → capture source) require SystemMixCapture capability

Graph visibility:
- All routes visible in AudioProofEvent log
- Mesh can visualize current graph (read-only from SexAudio)
- Harp displays user-relevant subset (does not show system-internal routes unless policy allows)
```

---

## 9. Capture/Recording Model

### Capture types (escalating risk):

| Type | Capability Required | Visibility | Indicator Required |
|------|--------------------|------------|-------------------|
| Self-capture (app captures own output) | SelfCapture | Proof log | No |
| Cross-capture (app captures another app) | CrossCapture | Proof log + Bell event | Yes (SilkBar) |
| System mix capture (record final mix) | SystemMixCapture | Proof log + Bell event + Collar grant | Yes (SilkBar) |
| Microphone capture | MicrophoneCapture | Proof log + Bell event + Collar grant | Yes (SilkBar) |

### Recording rules:
- RecordingSession must have explicit owner, route, privacy_class
- Privacy classes cannot be mixed in one recording (Public + Private → create separate sessions or reject)
- Device disappearance → RecordingSession tombstones → output file flushed if possible
- No silent recording — every active recording has a SilkBar indicator (shell-owned panel, not Harp)
- Recording stops when system suspends/shuts down (no background recording across power cycles in V1)
- Recording output goes to sexfiles via Linen refs — SexAudio does not own file storage
  - V1: recording output is proof-only (proof marker logged); sexfiles/Linen storage is future
  - V2: recording persisted through sexstore K/V as buffer reference; full sexfiles/Linen recording storage later

---

## 10. Virtual Device Model

### Virtual sink/source (BlackHole-class):
```
VirtualSink: app → [VirtualSink] → (internal buffer) → VirtualSource → approved recorder/app
```
- VirtualSink appears as an AudioDevice in AudioGraph
- Apps output to VirtualSink like any other device
- VirtualSource is the capture side — only explicitly authorized clients can connect
- Each virtual device pair has one owner (creator) who can grant access
- Virtual device disappears when owner disconnects or explicitly destroys it
- Destruction tombstones active streams but does not affect source material

### Virtual device capability:
- `VirtualDeviceCreate` capability required to create a virtual sink/source pair
- Grant from Collar required; grant specifies: owner, max_streams, privacy_class, persistence
- Virtual device cannot be used to bypass capture policy (capturing through virtual device still requires Capture capability for the source material)

---

## 11. Mixer/Priority/Ducking Policy

### Priority levels:
```
Critical (0): narration, alarms, security events — never ducked/dropped
High (1): media playback, voice communication
Medium (2): notification sounds, Bell events
Low (3): decorative/ambient sounds
Background (4): non-critical streams, idle tones
```

### Ducking rules:
- Critical streams: lower all other channels by configured amount (default -12dB) while active. Non-critical channels may be dropped entirely if mixer resources constrained.
- High streams: duck Medium and below.
- Medium streams: duck Low and below.
- Low/Background: no ducking authority.
- Ducking is automatic — no app control over duck parameters.
- Duck amounts are deterministic constants in SexAudio (not app-configurable in V1).

### Drop rules:
- If mixer channel resources exhausted (max streams per channel), lowest priority stream is dropped first.
- Within same priority: oldest stream dropped first.
- Critical streams never dropped.

### Queue rules:
- If target device busy with higher-priority stream, lower-priority stream is queued or rejected based on client preference.

### Volume/mute policy:
- Global volume: shell-owned, persisted through sexstore K/V.
- Per-stream volume: client may request gain in range [0, 1.0]. SexAudio enforces max gain per capability.
- Per-app mute: stored in sexstore K/V, applied in mixer.
- Media apps cannot set global volume or mute other apps.

---

## 12. Bell/Accessibility/Media Boundaries

### Bell integration:
- Bell emits AudioIntent for notification sounds (priority: Medium or High depending on event severity).
- Bell does not specify routing or mixing — SexAudio determines channel and ducking.
- Bell does not capture audio or access microphone.
- Bell sounds respect privacy class (Public for most notifications).

### Accessibility integration:
- D_ACCESSIBILITY_STACK emits narration intents (priority: Critical).
- Narration audio is routed through SexAudio like any other stream, but with Critical priority so it cannot be ducked or dropped.
- Narration content is not captured by system mix capture unless explicitly configured and privacy-warned.
- Accessibility narrations are marked with privacy class Narration — separate from Public/Media/Private.

### Media playback:
- Media apps request AudioIntent for playback with priority High.
- Media apps cannot set global routing — they output to default device unless user changes in Harp.
- Media apps cannot capture other apps or system audio without explicit grant.

---

## 13. Harp UI Model

### Harp is a control surface, not a mixer engine:
- Harp displays AudioGraph subset for the user's authorized scope
- User can see: connected devices, active streams, per-app volume/mute, active capture/recording sessions
- User can build routes: drag source node to sink node → Harp requests capability check → SexAudio validates → route created
- User can create virtual devices: fill form → Collar grant prompt → SexAudio creates

### Harp screens (V1):
1. **Route graph view** — visual graph of devices, apps, streams, mixers, virtual devices. Color-coded by state and privacy class.
2. **Per-app mixer** — list of audio clients with per-stream volume/mute sliders.
3. **Capture/record panel** — active capture/recording sessions with source, privacy class, duration. Start/stop controls.
4. **Virtual device manager** — list of virtual sinks/sources. Create/destroy. Access control.
5. **Presets** — save/load route configurations. Stored through sexstore K/V.

### Harp constraints:
- Harp cannot bypass SexAudio enforcement
- Harp cannot create routes without capability check
- Harp cannot start recording without Collar grant
- Harp displays capture warnings clearly (icon + text explaining what is being captured and why)
- Harp volume/mute changes are requests — SexAudio applies or rejects

---

## 14. Collar/Mesh/SilkBar/Quil Integration

### Collar:
- Mediates grants for: SelfCapture, CrossCapture, SystemMixCapture, MicrophoneCapture, VirtualDeviceCreate, Recording
- Grant specifies: scope (which apps/devices), privacy classes allowed, duration, persistence
- Revocation stops active sessions gracefully (SexAudio tombstones affected routes)

### Mesh:
- Read-only visualization of AudioGraph
- Shows: devices, routes, streams, capture sessions, virtual devices, tombstoned routes
- Color-codes: active (green), suspended (yellow), tombstoned (red), private (purple)
- Does not allow route modification — read-only diagnostic view

### SilkBar:
- Quick volume control (up/down/mute toggle) via shell panel
- Capture indicator: pulsing dot when any capture/recording is active
  - **V1 feasibility note:** SilkBar currently renders static panels. Dynamic capture indicator requires SilkBar panel protocol update. V1 may use proof marker only until SilkBar supports dynamic state
- Privacy warning: if recording includes private/narration content, indicator changes color/shape
- Shell-owned panel — Harp does not control SilkBar indicators

### Harp vs Mesh visualization scope:
- **Harp:** interactive control surface for user-relevant routes, per-app volume, capture/record controls. User can modify routes (subject to capability check).
- **Mesh:** read-only system-wide AudioGraph diagnostic view. Shows all routes, devices, streams, tombstoned nodes. Cannot modify anything.
- They are complementary, not duplicate: Harp is controls, Mesh is diagnostic.

### Quil:
- Inspects AudioProofEvent and RouteProofEvent logs
- Shows: every audio operation with client, target, capability, result
- Debug/dev view of AudioGraph state
- No runtime control of audio routing

---

## 15. Invariants

1. sexdisplay never owns audio capture, routing, mixing, or device control.
2. SexAudio enforces routing and capture policy; Harp only requests and configures — never bypasses enforcement.
3. Every sound/stream/tap/recording maps to an AudioClient and checked AudioCapability.
4. Every capture/recording session has visible owner, route, privacy class, and AudioProofEvent.
5. No hidden recording or loopback capture — every capture has a SilkBar indicator.
6. Apps cannot capture other apps by default — CrossCapture capability required.
7. System mix capture is higher-risk than self-capture and requires separate Collar grant.
8. Virtual devices require VirtualDeviceCreate capability and Collar grant.
9. Audio taps are capability-checked edges in AudioGraph — no unmonitored tapping.
10. Recording private/narration streams requires separate policy grant and privacy warning.
11. Accessibility narration has priority (Critical) over decorative/media sounds and cannot be ducked/dropped.
12. Bell sounds cannot drown narration unless policy explicitly allows (V1: narration always wins).
13. Media apps cannot own global volume/device routing — shell/Harp controls routing.
14. No raw cross-PD audio pointers — AudioBufferRef must go through SA7 transport gate.
15. Buffer transport must wait for approved copy/grant/ring design — no raw shared memory.
16. Device discovery does not imply playback readiness — state transition is explicit.
17. Missing hardware must fail silent/safe — no crash, no spin, no error flood.
18. Volume/mute policy is user/shell-owned, not app-owned — enforced by SexAudio.
19. Proof logs identify owner/type/route without leaking private audio content.
20. Harp cannot bypass Collar grants — grant check is in SexAudio, not Harp.
21. Mesh visualizes but does not grant or modify AudioGraph.
22. SilkBar shows quick controls and capture indicators but cannot authorize hidden capture.
23. AudioBufferRef must never expose a direct memory address across PD boundaries.

---

## 16. STOP FIRST Conditions

- Any kernel audio ABI edit
- Any PDX ABI edit for audio without explicit buffer transport gate
- Any raw shared audio buffer design without SA7 transport gate approval
- Any app direct device MMIO/DMA access
- Any POSIX ALSA/PulseAudio/PipeWire/CoreAudio assumptions
- Any std threads/sleep/timers for audio timing
- Any sexdisplay owning audio code or path
- Any Bell owning mixer or routing policy
- Any accessibility layer owning mixer or routing policy
- Any Harp owning enforcement instead of SexAudio
- Any media app owning global route or device selection
- Any microphone or system-audio capture before Collar policy exists
- Any hidden recording path — capture must have visible indicator
- Any app-to-app audio route without explicit capability
- Any virtual audio device without ownership and grant model
- Any zero-copy or ring-buffer design before transport gate
- Any broad device-driver refactor
- Any audio implementation before SA7 buffer transport gate is designed and approved

---

## 17. Proof Scenarios

1. No audio device exists → SexAudio boots silent/safe with empty AudioGraph.
2. Audio device discovered (sexusb) → SexAudio registers device → device appears in AudioGraph and proof log.
3. Bell emits low-priority notification sound intent → SexAudio accepts → routes to default output at Medium priority.
4. Accessibility narration intent emitted → SexAudio routes at Critical priority → ducks other channels.
5. Media app attempts stream without Output capability → rejected with proof event.
6. Trusted media app with Output capability → stream accepted into MixerChannel → plays through default device.
7. App attempts raw device access (bypasses SexAudio) → SexAudio rejects → STOP FIRST if kernel allows raw device MMIO access from userspace PD.
8. Mute enabled for media app → SexAudio suppresses non-critical output from that app.
9. App records its own output with SelfCapture grant → allowed, no indicator required.
10. App tries to record another app without CrossCapture grant → rejected with proof event and Bell security event.
11. User creates virtual sink/source in Harp → Collar grant checked → SexAudio creates virtual device → appears in AudioGraph.
12. App outputs to virtual sink → approved recorder with SystemMixCapture consumes virtual source.
13. System mix capture starts → SilkBar capture indicator appears → proof event logged.
14. Hidden capture attempt (no indicator, no grant) → SexAudio rejects → Bell security event emitted → Mesh shows failed attempt.
15. Device disappears during active recording → RecordingSession tombstones → output flushed if possible → client notified.
16. Bell sound plays during active recording → policy determines whether included (Public) or excluded (Private recording).
17. Accessibility narration during recording → privacy policy determines include/exclude/duck — logged in proof.
18. sexdisplay has no audio path touched — verified by static grep.
19. Harp requests route without Collar grant → SexAudio rejects → error displayed in Harp UI.
20. Proof log records owner/type/route without leaking private spoken content — verified by log inspection.

PROOF MARKERS:
```
[audio.client.register] client=N identity=S result=ok|denied
[audio.client.unregister] client=N
[audio.device.discover] device=N name=S type=output|input|virtual
[audio.device.lost] device=N reason=disconnect|failure
[audio.stream.open] stream=N client=N source=N sink=N priority=P privacy=C
[audio.stream.close] stream=N reason=normal|tombstone
[audio.route.create] route=N source=N sink=N capability=C result=ok|denied
[audio.route.teardown] route=N reason=normal|device_lost|capability_revoked
[audio.capture.start] session=N client=N source=N privacy=C indicator=1
[audio.capture.stop] session=N reason=normal|policy|tombstone
[audio.capture.denied] client=N target=N reason=no_capability|no_grant|privacy_mismatch
[audio.record.start] session=N client=N source=N privacy=C output=ref
[audio.record.stop] session=N reason=normal|tombstone|device_lost
[audio.virtual.create] device=N owner=N type=sink|source|pair result=ok|denied
[audio.virtual.destroy] device=N reason=owner_disconnect|explicit
[audio.mixer.gain] channel=N gain=G
[audio.mixer.mute] channel=N mute=1|0
[audio.mixer.duck] channel=N ducked_by=N amount=D
[audio.mixer.drop] channel=N stream=N reason=priority
[audio.error] reason=no_device|no_buffer_transport|capability_denied
```

---

## 18. Minimal Phase Ladder

1. **SA1_SEXAUDIO_AUDIT_V1** — Audit current audio stubs, services, device assumptions, IPC assumptions. No code.
2. **SA2_SEXAUDIO_PD_SKELETON_V1** — Create SexAudio PD with listener loop, slot registration, client registration skeleton. No audio logic — just the PD infrastructure.
3. **SA3_AUDIO_OBJECT_MODEL_SPEC_V1** — Define clients, intents, streams, graph nodes/edges, priorities, privacy classes. Handoff doc.
4. **SA4_AUDIO_CAPABILITY_POLICY_V1** — Define Collar grant model for output, capture, system mix, microphone, virtual devices. Handoff doc.
5. **SA5_AUDIO_DEVICE_ROUTE_DISCOVERY_V1** — Proof-only route/device visibility. No playback requirement.
6. **SA6_MIXER_POLICY_SPEC_V1** — Deterministic gain/mute/duck/drop/queue/priority policy. Handoff doc.
7. **SA7_AUDIO_BUFFER_TRANSPORT_GATE_V1** — Decide copy vs grant vs ring after proof. No raw shared buffer assumption. STOP FIRST if skipped.
8. **SA8_CAPTURE_RECORDING_VIRTUAL_DEVICE_SPEC_V1** — Audio Hijack/BlackHole-class features: taps, virtual sinks/sources, capture sessions. Handoff doc.
9. **SA9_BELL_ACCESSIBILITY_MEDIA_BOUNDARIES_V1** — Define Bell/accessibility/media intents and priority boundaries. Handoff doc.
10. **SA10_HARP_APP_MODEL_V1** — Harp UI/control model: graph builder, meters, routes, records, presets. Handoff doc.
11. **SA11_SEXAUDIO_HARP_PROOF_SCENARIOS_V1** — Deterministic proof scenarios and handoff.

---

## 19. Handoff Files

- `docs/handoff/SEXAUDIO_OBJECT_MODEL_V1.md` — full object model definitions
- `docs/handoff/AUDIO_CAPABILITY_POLICY_V1.md` — Collar grant model for audio
- `docs/handoff/AUDIO_GRAPH_CAPTURE_VIRTUAL_DEVICES_V1.md` — graph, capture, virtual device specs
- `docs/handoff/HARP_APP_MODEL_V1.md` — Harp UI/control model
- `docs/handoff/SEXAUDIO_PROOF_SCENARIOS_V1.md` — proof scenarios and results

---

## 20. Future Sub-Prompt Names

- `SA1_SEXAUDIO_AUDIT_V1`
- `SA2_SEXAUDIO_PD_SKELETON_V1`
- `SA3_AUDIO_OBJECT_MODEL_SPEC_V1`
- `SA4_AUDIO_CAPABILITY_POLICY_V1`
- `SA5_AUDIO_DEVICE_ROUTE_DISCOVERY_V1`
- `SA6_MIXER_POLICY_SPEC_V1`
- `SA7_AUDIO_BUFFER_TRANSPORT_GATE_V1`
- `SA8_CAPTURE_RECORDING_VIRTUAL_DEVICE_SPEC_V1`
- `SA9_BELL_ACCESSIBILITY_MEDIA_BOUNDARIES_V1`
- `SA10_HARP_APP_MODEL_V1`
- `SA11_SEXAUDIO_HARP_PROOF_SCENARIOS_V1`

---

## 21. Cross-Track Dependency Notes

- **Bell:** may emit audio intents (notification sounds) but SexAudio mixes/routes. Bell does not control routing or capture.
- **D_ACCESSIBILITY_STACK:** may emit narration intents (Critical priority) but SexAudio routes/mixes. Accessibility does not control mixer policy.
- **Harp:** may request routes and control per-app volume, but SexAudio enforces all operations.
- **Collar:** owns grants for SelfCapture, CrossCapture, SystemMixCapture, MicrophoneCapture, VirtualDeviceCreate, Recording. SexAudio checks grants before any operation.
- **Mesh:** may visualize AudioGraph and failure events (read-only from SexAudio). Does not modify routes.
- **SilkBar:** shows capture indicators and quick volume control (shell-owned panel, not Harp). Cannot authorize capture.
- **Quil:** inspects AudioProofEvent and RouteProofEvent logs. No runtime control.
- **Linen/sexfiles:** stores recordings and audio projects later. Does not mix, play, or route audio.
- **sexdisplay:** remains pixels only — no audio code, no audio path, no audio visualization in framebuffer.
- **sexusb/sexpci:** hardware bus enumeration for audio devices. SexAudio consumes device discovery events but does not manage hardware.
