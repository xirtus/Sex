# THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1

**Status:** Plan only. No implementation.

**Theremin rule (recurring):** No sound engine feature should imply route ownership, recording, capture, or device access. Theremin only maps typed events to deterministic sound intents/presets; SexAudio owns routing/mixing/device/capture.

**Scope warning:** Theremin is the deterministic physical-modeling system sound engine for SexOS — a tiny no_std physical-model sound intent engine, not a synth workstation, asset player, or audio route owner. It turns typed system events into tiny instrument-like audio intents without MP3s, codecs, asset packs, filesystem dependency, or hidden playback paths. Theremin never touches audio hardware directly. Theremin emits only ThereminIntent or approved PCM/handle to SexAudio. SexAudio owns mixer, device, and routing. Theremin cannot create audio routes, cannot record, and cannot capture.

---

## 1. Mission

MISSION: THEREMIN_SYSTEM_SOUND_ENGINE_PLAN_V1 — Design Theremin, the deterministic physical-modeling system sound intent engine for SexOS: boot, shutdown, notification, error, denied, security, recording, device, gesture, focus, and shell event sounds. Theremin emits only ThereminIntent or approved PCM/handle to SexAudio. SexAudio owns mixer, device, and routing. Theremin never synthesizes to hardware directly. Docs/plan only. No implementation.

---

## 2. Why Theremin Exists

System sounds in traditional OSes are either:
- MP3/WAV asset files requiring a codec, filesystem, and asset management pipeline
- Beep/sine generators that sound primitive and indistinguishable
- Hidden in the window manager or compositor with no capability scoping

Theremin solves this by treating system sounds as **deterministic physical-modeling intents**: each sound is synthesized from a tiny set of parameters (exciter type, resonator model, pitch curve, damping), scoped to a typed system event, controlled by Settings policy, routed by SexAudio, and explainable through proof logs.

Theremin is separate from SexAudio because:
- SexAudio is routing/mixing/capture authority — not a sound generator
- Harp is user control surface — not a sound engine
- Bell emits notification intents — not raw audio or synth parameters
- Theremin is the **synth engine** — it produces audio intents/PCM from event types

---

## 3. Dependency Gates

1. SexAudio must exist and accept Theremin audio intents before Theremin can produce system sounds.
2. SexAudio buffer transport (SA7) must be approved before Theremin sends PCM to SexAudio.
3. Bell must be able to emit typed notification events before Theremin maps Bell events to sounds.
4. Settings persistence (sexstore K/V) must exist for Theremin policy (enable/disable, volume, theme).
5. No Theremin implementation before T5 no_std synth gate reviews fixed-point/timing/CPU/memory constraints.
6. **Critical: Tick source dependency** — Theremin requires a deterministic tick/sample source from shell/input event loop. If no approved source exists by T4 handoff, Theremin must ship with **proof-marker-only mode** (no audible output) until source is approved. No fallback to std timers, PC speaker, or hardware beep.
7. **Critical: SexAudio deadlock guard** — Theremin design must be independently deliverable in proof-marker-only mode (all proof events, no audio output) if SexAudio buffer transport (SA7) is delayed. Proof-marker-only mode exercises the full event/policy/preset pipeline without SexAudio dependency.

---

## 4. Theremin vs SexAudio vs Harp vs Bell Ownership

### Theremin owns:
- System sound theme (mapping of SystemSoundEvent to PhysicalModelPreset)
- Event-to-sound mapping (typed event → synth parameters)
- Physical-modeling presets (parameter sets for each instrument model)
- Tiny deterministic synth algorithms (exciter + resonator + envelope per voice)
- Category enable/disable policy hooks
- Sound intent generation (ThereminIntent sent to SexAudio)
- Sound proof events (ThereminProofEvent)

### SexAudio owns:
- Audio device output, mixing, routing, capture
- Receives ThereminIntents and routes to appropriate output/mixer channel
- Applies gain/mute/duck/priority per SexAudio policy
- Theremin does not bypass SexAudio mixing or routing

### Harp owns:
- User-facing audio graph UI — not sound engine settings
- Theremin system sounds appear as a SexAudio client in Harp's per-app mixer
- Harp does not configure Theremin presets or themes

### Bell owns:
- Notification event policy (when to notify, visual vs audible, priority)
- Bell emits typed events — Theremin maps events to sounds
- Bell does not generate audio or control Theremin presets

### Neither owns:
- Audio device/hardware: sexusb/sexpci
- Framebuffer/pixels: sexdisplay
- Trust/grants: Collar
- Global graph visualization: Mesh
- Quick controls/indicators: SilkBar (may show mute/volume for Theremin as a SexAudio client)
- Settings UI: Silk settings panels (Theremin provides policy hooks; settings UI is shell-owned)
- File/project storage: Linen/sexfiles

---

## 5. Innovation Goal

SexOS system sounds should feel like a tiny living instrument built into the OS. Every sound is generated from deterministic physical-model parameters, scoped to a typed system event, controlled by settings, routed by SexAudio, and explainable through proof logs.

No MP3s, no codecs, no asset packs, no filesystem dependency for boot/error sounds. Every boot, every notification, every error is a tiny instrument performance.

---

## 6. Object Model

- **SystemSoundEvent:** a typed event that triggers a system sound. Has event_kind (Boot, Shutdown, Notification, Error, etc.), category (SoundCategory), timestamp (tick count), optional metadata (severity u8, source).
- **SoundCategory:** a group of system sounds with shared policy (enabled, volume_u8, priority, privacy_class, route_hint). Categories: Boot, Shutdown, Login, Logout, Notification, Warning, Error, Critical, Success, Denied, Focus, Gesture, Window, Screenshot, RecordingStart, RecordingStop, DeviceConnected, DeviceDisconnected, LowPower, SecurityAlert.
- **SoundTheme:** a named collection of PhysicalModelPresets mapped to SystemSoundEvents. V1: one built-in theme (default). Future: user-created themes.
- **PhysicalModelPreset:** a parameter set defining one synthesized sound. Contains instrument_model, exciter_params, resonator_params, envelope, damping (u8 0..255), brightness (u8 0..255), pitch_curve, amplitude_curve, max_duration_ticks. All values are integer or fixed-point — no float in V1 unless explicitly marked future.
- **InstrumentModel:** the type of physical-modeling algorithm. V1 presets: GlassBell, CrystalPluck, SoftWoodTick, MetalWarning, DeniedKnock, BootRise, ShutdownFall, DevicePing, SecurityPulse, RecordingPulse.
- **Exciter:** the attack/noise component of the synth (impulse, noise burst, filtered noise, swept sine). Parameters: type, intensity (u8 0..255), duration_ticks (u32), brightness (u8 0..255).
- **Resonator:** the tonal body component (modal bank, waveguide, damped oscillator, inharmonic bank). Parameters: model_type, frequencies (integer Hz or fixed-point pitch_milli_hz), damping_per_mode (u8 0..255 per mode), brightness (u8 0..255).
- **ModalBank:** a resonator consisting of N sine oscillators at specified frequencies with per-mode damping and gain. N bounded per preset (V1: max 6 for GlassBell, max 4 for MetalWarning, max 2 for simple presets). Frequencies in integer Hz or fixed-point pitch_milli_hz.
- **Waveguide:** a Karplus-Strong-style resonator with fixed-size delay line (length = integer sample count), feedback gain (u8 0..255), filtering. Delay line size is bounded and declared per preset — STOP FIRST if dynamic allocation proposed.
- **Envelope:** amplitude envelope (attack, decay, sustain_level, release). All times in integer tick counts (u32). Sustain level and gain are u8 0..255.
- **Damping:** per-frequency or per-mode damping coefficient (u8 0..255 scale, 0 = no damping, 255 = fully damped).
- **Brightness:** spectral emphasis (u8 0..255, mapped to filter cutoff or mode gain distribution via fixed-point table).
- **PitchCurve:** pitch trajectory over sound lifetime (start_freq_hz, end_freq_hz, curve_shape, duration_ticks). Frequencies in integer Hz or fixed-point pitch_milli_hz. No float.
- **AmplitudeCurve:** amplitude trajectory (start_gain_u8, end_gain_u8, curve_shape, duration_ticks). Gain is u8 0..255.
- **ThereminIntent:** a request from Theremin to SexAudio to play a sound. Contains intent_kind (play_sound, silence_category), category, preset_id, priority, privacy_class, estimated_duration_ticks, optional PCM buffer/handle (if SA7 transport approved). Theremin never sends intent directly to hardware.
- **ThereminPolicy:** deterministic rules for which categories are enabled, volume_u8 per category (0=mute, 255=full), priority per category (maps to AudioPriority 0..4), privacy_class override (0=Public..5=Secure), route_hint (0=default, 1=all, 2=headphones, 3=virtual). Policy is shell/settings-owned, not app-owned.
- **ThereminProofEvent:** logged proof event for any Theremin operation. Contains event_kind, category, preset_id, policy_result (played, silenced, disabled, denied, dropped). Proof logs record only category/preset/result — no private content, no message text, no document names.

---

## 7. System Sound Category Model

Each category supports:
- `enabled: bool` — if false, category produces silence and `[theremin.silence]` proof marker
- `volume_u8: u8` — 0=mute, 255=full. Integer only — no float volume scalar (no 0.0..1.0). Applied before sending to SexAudio
- `priority: u8` — maps to AudioPriority: 0=Critical(narration), 1=High(media), 2=Medium(notifications), 3=Low(ambient), 4=Background
- `privacy_class: u8` — maps to AudioPrivacyClass: 0=Public, 1=Media, 2=Narration, 3=Communication, 4=Private, 5=Secure
- `route_hint: Option<RouteHint>` — optional suggestion to SexAudio for output channel (default device, headphones, virtual sink)

### Category table (V1):

| Category | Default | Priority | Privacy | Route hint |
|----------|---------|----------|---------|------------|
| Boot | enabled | High | Public | default |
| Shutdown | enabled | High | Public | default |
| Login | enabled | Medium | Public | default |
| Logout | enabled | Medium | Public | default |
| Notification | enabled | Medium | Public | default |
| Warning | enabled | High | Public | default |
| Error | enabled | High | Public | default |
| Critical | enabled | Critical | Public | default+all |
| Success | enabled | Medium | Public | default |
| Denied | enabled | Medium | Public | default |
| Focus | enabled | Low | Public | default |
| Gesture | enabled | Low | Public | default |
| Window | enabled | Low | Public | default |
| Screenshot | enabled | Low | Public | default |
| RecordingStart | enabled | Medium | Narration | default+indicator |
| RecordingStop | enabled | Medium | Narration | default |
| DeviceConnected | enabled | Low | Public | default |
| DeviceDisconnected | enabled | Low | Public | default |
| LowPower | enabled | High | Public | default+all |
| SecurityAlert | enabled | Critical | Secure | default+all |

---

## 8. Physical-Modeling Synth Model (V1)

### V1 presets:

| Preset | Instrument Model | Exciter | Resonator | Max Modes/Buffer | Character |
|--------|-----------------|---------|-----------|-----------------|-----------|
| GlassBell | ModalBank (inharmonic) | Impulse | Damped modal | Max 6 modes | Bright attack, long tail |
| CrystalPluck | Waveguide (Karplus-Strong) | Noise burst | Filtered delay line | Fixed-size delay line (declare at compile time) | Bright pluck, pitch bend down |
| SoftWoodTick | ModalBank (damped) | Filtered noise (bounded burst) | Heavy damping, 1-2 modes | Max 2 modes, bounded noise burst | Short, soft, warm |
| MetalWarning | ModalBank (inharmonic) | Swept sine | Inharmonic modes | Max 4 inharmonic modes | Harsh, metallic, medium |
| DeniedKnock | ModalBank (low) | Impulse | Heavy damping | Max 2 modes | Short, low, dull thud |
| BootRise | ModalBank (rising) | Swept sine rising | Rising pitch | Max 4 modes | Ascending, hopeful |
| ShutdownFall | ModalBank (falling) | Impulse | Descending pitch | Max 4 modes | Descending, calm |
| DevicePing | ModalBank (tuned) | Impulse | Light damping | Max 2 modes | Short, bright ping |
| SecurityPulse | ModalBank (interval) | Impulse repeated (bounded) | Pulsing | Max 2 modes, bounded repeat count | Low-high alternating |
| RecordingPulse | ModalBank (soft) | Noise burst (rate-limited) | Very light | Max 1 mode, rate-limited (never constant loop unless policy allows) | Soft periodic marker |

Per-preset limits are compile-time constants, not runtime configuration. Each preset declares max modes, max buffer size (in samples), and max duration (in ticks) at compile time. STOP FIRST if any preset exceeds 44,100 samples (max 2×sample_rate at 22050 Hz) or requires dynamic allocation.

### Synth algorithm structure (per voice):
```
1. Exciter produces N samples (impulse, noise, swept sine)
2. Resonator processes exciter output through modal bank or waveguide
   - Modal bank: N sine oscillators summed, each with frequency, damping, gain
   - Waveguide: delay line with filtered feedback
3. Envelope shapes amplitude (attack, decay, sustain, release in tick counts)
4. Pitch curve modulates resonator frequencies over time
5. Brightness curve modulates damping or filter
6. Output is integer (i16) PCM samples in bounded buffer
```

All parameters are integer or fixed-point (no float in V1 unless no_std target safely supports it). All durations are in integer tick counts at a deterministic sample rate. Bounded memory: max buffer size = sample_rate * max_duration_ticks per voice.

### Reference Guidance (inspiration only, not direct port)

- **Faust physical modeling libraries:** Study algorithm patterns (modal synthesis, waveguide, exciter+resonator architecture). Do NOT port Faust code directly — extract small algorithm concepts for native no_std Rust.
- **RipplerX / modal synths:** Understand modal bank design (per-mode frequency, damping, gain). Adapt for bounded integer/fixed-point parameter tables.
- **Karplus-Strong / string synths:** Study fixed-delay-line pluck synthesis. Adapt with fixed-size buffers, no heap allocation.
- **Modal resonator designs:** Understand how mode counts, inharmonic ratios, and damping produce different timbres with minimal parameters.

**Constraint:** All inspiration must be extracted as small, bounded, no_std-compatible algorithms. No heap-heavy DSP framework. No direct port of Faust/JUCE/VST/plugin code. No floating-point dependency without verifying target support.

---

## 9. Theme/Preset Model

- **SoundTheme:** a mapping of SystemSoundEvent → PhysicalModelPreset
- V1: one built-in theme "Default" — hardcoded in Theremin source code
- Future: user may select from built-in themes or create through settings UI
- Theme change: Theremin reloads preset mapping; does not affect SexAudio or active playback
- Theme validation: every SystemSoundEvent must map to a valid preset. Invalid = fallback to silent/default.
- Theme persistence: through sexstore K/V (key range reserved for Theremin settings)

---

## 10. Settings Policy Model

Settings storage: sexstore K/V (same as scene appearance settings pattern).
Shell-owned settings UI (Silk settings panel) reads/writes ThereminPolicy through Theremin.

Policy keys (sexstore K/V):
- `THEREMIN_KEY_ENABLED: u64 = 0x10` — master enable/disable (bool as u8)
- `THEREMIN_KEY_VOLUME: u64 = 0x11` — master volume (u8 0..255)
- `THEREMIN_KEY_THEME: u64 = 0x12` — selected theme ID (u8)
- `THEREMIN_KEY_CATEGORY_PREFIX: u64 = 0x20` — per-category settings start

Per-category settings (stored as u64 blob per category):
```
Byte 0: enabled (bool)
Byte 1: volume (u8)
Byte 2: priority (u8)
Byte 3: privacy_class (u8)
Byte 4: route_hint (u8, 0=default, 1=all, 2=headphones, 3=virtual)
Byte 5-7: reserved
```

---

## 11. Bell/Silk/SexAudio Integration

### Bell → Theremin flow:
```
Bell emits typed notification event → shell receives event → shell forwards to Theremin
→ Theremin maps event to SystemSoundEvent → maps event to PhysicalModelPreset
→ Theremin checks policy (enabled, volume, priority, privacy)
→ Theremin generates ThereminIntent → sends to SexAudio
→ SexAudio routes intent through mixer → plays through device
```

### SilkBar integration:
- SilkBar may show Theremin mute/volume quick control (shell-owned panel)
- SilkBar does not configure Theremin presets or themes
- Theremin appears as a SexAudio client in the audio graph (visible in Harp mixer)

### SexAudio integration:
- Theremin sends ThereminIntent to SexAudio via PDX IPC
- ThereminIntent contains: category, priority, privacy_class, estimated_duration_ticks, optional PCM buffer (if SA7 transport approved)
- SexAudio treats Theremin as an audio client with pre-configured capabilities
- SexAudio applies gain/mute/duck/priority per mixer policy
- SexAudio may drop Theremin intent if higher-priority audio is active (e.g., narration)

---

## 12. no_std Timing/Generation Constraints

- No std::time, thread::sleep, or POSIX timer APIs
- All timing uses deterministic tick counters from shell/input event loop — **STOP FIRST if no approved deterministic tick/sample source exists**
- Sample rate is a fixed integer constant (e.g., `SAMPLE_RATE_HZ: u32 = 22050`) — not device-configurable in V1
- Synth generation is synchronous: Theremin generates a sound buffer in response to an event, sends to SexAudio, and returns
- No background synthesis thread — all generation is event-driven in the PDX message loop
- Buffer size per sound is bounded: max 44,100 samples at sample_rate (2×sample_rate at 22050 Hz)
- Fixed-point arithmetic for all synthesis parameters (no float in V1 unless target already safely supports it)
- Integer lookup tables for sine/triangle/saw waves (pre-computed, no runtime math library)

---

## 13. Invariants

1. No MP3/WAV/codec/asset dependency for V1 system sounds — all sounds are synthetically generated.
2. No filesystem dependency for boot, error, or security sounds — presets are hardcoded constants.
3. Theremin never touches audio hardware directly — all output through SexAudio.
4. Theremin sends sound intents or PCM only through SexAudio — no direct device path.
5. Theremin obeys Settings and Bell policy — disabled categories produce silence.
6. No hidden playback path — every ThereminIntent is logged as a ThereminProofEvent.
7. Critical/security sounds must NOT bypass mute or privacy in V1. No bypass is permitted under any circumstances. A future policy change that permits bypass must be approved through the full capability model (Collar grant + SexAudio policy + shell policy), not by Theremin alone.
8. Sound generation is deterministic from preset parameters and event type — same event + same preset = same output.
9. Proof logs identify event/category/preset, not private content.
10. Missing SexAudio or no audio device → Theremin intent safely dropped — no crash, no spin.
11. No std threads/sleep/time APIs for sound generation or timing.
12. Synth timing uses existing deterministic tick/sample model only — no wall-clock time.
13. Physical models must be bounded CPU and memory — max N modes, max buffer size, max simultaneous voices.
14. Recording sounds must integrate with capture indicator and privacy policy — RecordingStart/Stop events respect recording state.
15. Theme/preset changes do not affect active playback — only affect future sounds.
16. Unknown/invalid preset mapping → fallback to silent/default — no crash.
17. Every generated sound must be associated with a category, a preset, and a policy decision (allowed/muted/disabled/dropped).
18. Disabled category produces no audio intent — only a `[theremin.policy.mute]` proof marker.
19. Theme change must be deterministic and reversible. The same theme + same event always produces the same preset mapping.
20. Theme cannot introduce unapproved code, dynamic loading, or asset decoders. V1 themes are hardcoded preset tables.
21. User sound settings (enable/disable, volume, theme) are shell/settings-owned, not app-owned. Theremin reads policy from sexstore K/V; it does not expose policy writes to arbitrary apps.
22. Theremin cannot create audio routes, cannot own audio devices, cannot record, and cannot capture audio. All routing, device ownership, recording, and capture are in SexAudio domain.
23. Harp may preview Theremin presets but only through SexAudio policy — Harp cannot bypass SexAudio mixing, ducking, or privacy enforcement for previews.
24. **Polyphony bound:** V1 max simultaneous voices = 2. If voice limit is reached, the oldest running voice is dropped and the new voice starts. Proof marker logged on drop (`[theremin.intent.drop]` reason=too_many_voices).
25. **Volume precision:** Volume scaling chain (master_volume_u8 × category_volume_u8 × envelope_gain_u8) must use fixed-point arithmetic with at least 16-bit intermediate precision. Division/truncation must not produce audible quantization steps.
26. **Recording mix exclusion:** RecordingStart/Stop sounds must be excluded from the recording capture mix by SexAudio policy. Theremin proof marker verifies this exclusion.
27. **Proof log timing safety:** Proof log timestamps use shell tick count only (no wall clock). Logs must NOT record event frequency patterns that could reveal user behavior beyond the category/preset already logged.
28. **Pitch precision:** `pitch_milli_hz` is u32 with 1/1000 Hz resolution (range 0..4,294,967,295 milli-Hz). This covers the full audible range with integer precision. Frequency calculations must use integer arithmetic only — no float intermediate for pitch.
29. **Sample rate conversion:** If SexAudio operates at a different sample rate than Theremin's fixed 22050 Hz, Theremin must perform rate conversion (not SexAudio). Conversion must use integer linear interpolation or sample-and-hold — no float resampling.

---

## 14. STOP FIRST Conditions

- Any MP3/WAV decoder dependency proposed for V1
- Any filesystem dependency for boot, error, or security sounds
- Any direct audio hardware access from Theremin
- Any bypass of SexAudio mixer/routing — Theremin must not own audio output
- Any std audio library, thread, sleep, or time dependency
- Any POSIX/CoreAudio/ALSA/PulseAudio/PipeWire assumptions
- Any direct port of Faust/JUCE/VST/plugin code without native no_std rewrite
- Any heap-heavy DSP framework
- Any unbounded resonator/modal bank count
- Any sound setting that bypasses Bell or Settings policy
- Any security/critical sound that bypasses mute/privacy without explicit policy
- Any private event content stored in proof logs
- Any floating-point DSP dependency without verifying no_std target support
- Any real-time synthesis thread or background audio generation (V1 is synchronous event-driven only)
- Any sample rate above 22050 Hz for V1 without verifying CPU budget
- Any tick/sample timing dependency without an approved deterministic tick/sample source
- Any attempt to use Theremin as an audio route owner, recorder, or capture source — Theremin emits intents only; SexAudio owns routes, recording, and capture
- Any theme/preset that requires dynamic code loading, asset decoders, or filesystem access at runtime
- Any floating-point arithmetic in synth DSP, including but not limited to: fractional delay line interpolation, modal frequency calculation, envelope evaluation, pitch curve evaluation. All DSP must be integer or fixed-point only — "no float" means no float operations at all in the synthesis hot path, not merely "no float library."
- Any boot/error/critical sound fallback path that bypasses SexAudio (PC speaker, PIT beep, initramfs audio driver, direct MMIO). Boot sounds must either emit through SexAudio after SexAudio is ready, or be deferred — never direct-hardware.

---

## 15. Proof Scenarios

### Proof markers

```
[theremin.event.recv] event=Boot|Shutdown|... category=N source=S
[theremin.policy.allow] category=N preset=N volume=G
[theremin.policy.mute] category=N reason=disabled|volume_zero|bell_policy
[theremin.preset.select] preset=N instrument=GlassBell|...
[theremin.preset.reject] preset=N reason=invalid|corrupt|unsupported
[theremin.intent.emit] intent=N category=N preset=N priority=P duration_ticks=D
[theremin.intent.drop] reason=no_sexaudio|sexaudio_busy|policy_denied
[theremin.theme.change] theme=N result=ok|rejected
[theremin.error] reason=no_tick_source|buffer_overflow|invalid_parameter
```

### Scenarios

1. Boot sound enabled → BootRise preset selected → `[theremin.preset.select]` → `[theremin.policy.allow]` → `[theremin.intent.emit]` → SexAudio receives and routes.
2. Boot sound disabled (settings) → `[theremin.policy.mute]` reason=disabled → no intent emitted → `[theremin.intent.emit]` absent.
3. Error event (no audio device) → MetalWarning generated → `[theremin.intent.emit]` → SexAudio drops safely → `[theremin.intent.drop]` reason=no_sexaudio.
4. Denied action → DeniedKnock emitted → proof log records category=Denied preset=DeniedKnock — does NOT record the denied action content.
5. Notification muted (Bell policy) → Theremin checks policy → `[theremin.policy.mute]` reason=bell_policy → no intent.
6. Recording start → RecordingStart sound + capture indicator (SilkBar) both active → `[theremin.event.recv]` + `[theremin.intent.emit]`.
7. Recording category disabled → capture indicator remains, RecordingStart sound silent → `[theremin.policy.mute]` reason=disabled.
8. Device connected → DevicePing → `[theremin.intent.emit]` preset=DevicePing.
9. Device disconnected → ShutdownFall or disconnect preset.
10. No SexAudio available → Theremin intent safely dropped → `[theremin.intent.drop]` reason=no_sexaudio → no crash, no spin.
11. Theme changed → same event maps to different preset → different deterministic output → `[theremin.theme.change]` result=ok.
12. Invalid preset mapping (corrupt/unknown) → `[theremin.preset.reject]` → fallback to silent/default → no intent.
13. SecurityAlert → obeys privacy/mute policy as specified — `[theremin.policy.allow]` if policy allows, else `[theremin.policy.mute]`.
14. Theremin tries direct device route (bypasses SexAudio) → STOP FIRST violation detected at design/audit time.
15. Multiple events in rapid succession → bounded voice count → oldest voice dropped if max exceeded → `[theremin.intent.drop]` reason=too_many_voices.
16. Proof log inspected → contains category/preset only, no private event payload → verified by log inspection.
17. Muted category → no SexAudio intent → `[theremin.policy.mute]` reason=volume_zero.
18. Invalid preset parameter (out of bounds) → `[theremin.preset.reject]` → fallback silent/default.
19. Boot sound without filesystem → GlassBell preset from compiled constants → `[theremin.preset.select]` preset=BootRise.
20. No deterministic tick/sample source exists → STOP FIRST gap documented in T5 gate.
21. Harp preview request without SexAudio policy → `[theremin.policy.mute]` → no preview intent.
22. RecordingPulse during active recording → rate-limited per policy → `[theremin.intent.emit]` with bounded rate.
23. Theme attempts to load file/decoder dependency → rejected for V1 → `[theremin.theme.change]` result=rejected.
24. Critical sound while mute enabled → obeys current policy → `[theremin.policy.mute]` unless future policy explicitly allows bypass.
25. Proof logs exclude private content → verified: intent emits category/preset only, no message text, no document names.
26. Tick source unavailable → Theremin operates in proof-marker-only mode → `[theremin.policy.mute]` for all categories → no audio emitted → proof events still logged normally.

---

## 16. Minimal Phase Ladder

1. **T1_THEREMIN_AUDIT_V1** — Audit current Bell/hooks, settings storage, SexAudio boundary, existing sound stubs. No code.
2. **T2_SYSTEM_SOUND_EVENT_MODEL_V1** — Define SystemSoundEvent, SoundCategory, ThereminPolicy, ThereminProofEvent. Handoff doc.
3. **T3_THEREMIN_POLICY_SETTINGS_SPEC_V1** — Define sexstore K/V key layout, category settings format, policy evaluation rules. Handoff doc.
4. **T4_THEREMIN_SEXAUDIO_BOUNDARY_V1** — Define ThereminIntent message format, PDX opcode, integration with SexAudio client model. Handoff doc.
5. **T5_THEREMIN_NO_STD_SYNTH_GATE_V1** — Gate: review fixed-point arithmetic, integer tables, tick-based timing, bounded buffer, CPU/memory budget. **STOP FIRST if constraints not met** — this gate runs BEFORE detailed preset spec to ensure all physical models fit within no_std constraints.
6. **T6_PHYSICAL_MODEL_PRESET_SPEC_V1** — Define all 10 V1 presets with exact integer/fixed-point parameters (exciter type, resonator model, max modes, envelope in ticks, pitch curve in Hz/milli-Hz). Must pass T5 gate constraints. No implementation — spec only.
7. **T7_THEREMIN_BELL_SILK_INTEGRATION_V1** — Define Bell event → Theremin mapping, SilkBar mute/volume quick control hooks. Handoff doc.
8. **T8_THEREMIN_PROOF_SCENARIOS_V1** — Deterministic proof plan covering all 26 scenarios with proof markers.

---

## 17. Handoff Files

- `docs/handoff/THEREMIN_SOUND_EVENT_MODEL_V1.md` — event model, categories, policies (T2)
- `docs/handoff/THEREMIN_POLICY_SETTINGS_V1.md` — settings K/V layout, category settings format (T3)
- `docs/handoff/THEREMIN_SEXAUDIO_BOUNDARY_V1.md` — SexAudio integration, ThereminIntent message format (T4)
- `docs/handoff/THEREMIN_NO_STD_SYNTH_GATE_V1.md` — fixed-point/tick/buffer gate approval (T5)
- `docs/handoff/THEREMIN_PHYSICAL_MODEL_PRESETS_V1.md` — preset parameter tables with integer/fixed-point values (T6)
- `docs/handoff/THEREMIN_BELL_SILK_INTEGRATION_V1.md` — Bell/SilkBar hooks (T7)
- `docs/handoff/THEREMIN_PROOF_SCENARIOS_V1.md` — proof plan with markers (T8)

---

## 18. Future Sub-Prompt Names

- `T1_THEREMIN_AUDIT_V1`
- `T2_SYSTEM_SOUND_EVENT_MODEL_V1`
- `T3_THEREMIN_POLICY_SETTINGS_SPEC_V1`
- `T4_THEREMIN_SEXAUDIO_BOUNDARY_V1`
- `T5_THEREMIN_NO_STD_SYNTH_GATE_V1`
- `T6_PHYSICAL_MODEL_PRESET_SPEC_V1`
- `T7_THEREMIN_BELL_SILK_INTEGRATION_V1`
- `T8_THEREMIN_PROOF_SCENARIOS_V1`

---

## 19. Cross-Track Dependency Notes

- **SexAudio:** Theremin sends intents to SexAudio. SexAudio must exist and accept Theremin client.
- **Harp:** Theremin appears as a SexAudio client in Harp's per-app mixer. Harp has no Theremin-special logic.
- **Bell:** Bell emits typed events that Theremin maps to sounds. Theremin does not replace Bell.
- **SilkBar:** May show Theremin mute/volume quick control. SilkBar does not configure Theremin presets.
- **Settings (shell):** Settings panel reads/writes ThereminPolicy through sexstore K/V. Theremin does not own settings UI.
- **Mesh:** Theremin appears as a node in the audio graph (read-only). Mesh does not control Theremin.
- **Quil:** Inspects ThereminProofEvent logs. No runtime control.
- **sexdisplay:** No integration — Theremin has no visual output.
- **Collar:** No grants needed for system sounds in V1. Future: user-created themes may need Collar policy.
- **Linen/sexfiles:** No integration — Theremin has no file storage dependency for V1.
- **sexusb/sexpci:** No integration — Theremin does not discover or manage audio hardware.

---

## 20. Premortem Analysis

**Premise:** Assume this plan failed 6 months after acceptance. Below are the identified failure modes, their categories, and the revised safest path hardening applied above.

### Failure Mode Table

| # | Failure Mode | Category | Severity | Hardening Applied |
|---|-------------|----------|----------|-------------------|
| 1 | Tick source never built → Theremin has no timing foundation | Invariant violation (§13.12) | **Critical** | §3.6: proof-marker-only mode if no source; STOP FIRST §14.19; §20.12 new scenario |
| 2 | Float creep into synth algorithms (fractional delay, modal freq, envelope curve) | Invariant violation (§13 "no float") | **Critical** | STOP FIRST §14.20: no float operations in synthesis hot path at all |
| 3 | SexAudio deadlock (SA7 delayed) → Theremin indefinitely blocked | Scope creep / dependency stall | **Critical** | §3.7: proof-marker-only mode independently deliverable; §15 scenario 20 extended |
| 4 | Boot sound bypass via PC speaker/PIT/initramfs | Renderer ownership / MPK fault (§13.3) | **Critical** | STOP FIRST §14.21: no direct-hardware fallback for any boot/error/critical sound |
| 5 | Performance budget unvalidated → generation blocks PDX message loop | MPK/PDX fault (§13.13) | **High** | §13.24: max voices=2; §13.13 already requires bounded CPU; T5 gate verifies ticks per generation |
| 6 | Polyphony unbounded → OOM or CPU spiral | Invariant violation (§13.13) | **High** | §13.24: V1 max 2 voices, oldest dropped |
| 7 | Security/critical bypass loophole ("unless explicitly designed") | Invariant violation (§13.7) | **High** | §13.7 tightened: NO bypass permitted in V1 |
| 8 | Volume u8×u8 multiplication loses precision → audible steps | Implementation defect | **Moderate** | §13.25: 16-bit intermediate precision required |
| 9 | Recording sound feedback (Start/Stop sounds captured by recording) | System integration fault (§13.14) | **Moderate** | §13.26: excluded from capture mix by policy |
| 10 | Proof log timing side channel (frequency patterns reveal behavior) | Privacy leak (§13.9) | **Moderate** | §13.27: no wall clock, no frequency pattern logging |
| 11 | Sample rate mismatch (22050 vs 44100) forces unplanned SRC | ABI drift | **Moderate** | §13.29: Theremin does SRC via integer interpolation |
| 12 | Theme becomes de facto plugin system | Scope creep (§13.20) | **Moderate** | §13.20 already prohibits; premortem confirms no runtime theme loading in V1 |
| 13 | PitchCurve float pressure (261.63 Hz = non-integer) | Invariant violation (§6 PitchCurve) | **Low** | §13.28: pitch_milli_hz u32 at 1/1000 resolution; §6 uses integer Hz or milli-Hz |
| 14 | PCM buffer transport bypasses MPK (raw pointer instead of SA7 handle) | MPK/PDX fault (§13.22) | **Low** | Already gated by SA7; premortem confirms handle-only, no raw pointers |

### Revised Safest Path Summary

1. **Proof-marker-only mode** must be the first deliverable — exercises full event/policy/preset/proof pipeline without any SexAudio dependency. This decouples Theremin from SA7 and tick source timelines.
2. **Zero-float DSP constraint** must be enforced at T5 gate and verified by code review. Fractional delay = integer interpolation or sample-and-hold, not float.
3. **No hardware fallback ever** — boot sounds are either SexAudio-routed or silent. No PC speaker, no PIT beep, no initramfs audio.
4. **All bounds declared at compile time** — max voices (2), max modes per preset, max samples per sound, max ticks per generation. No runtime configuration of bounds.
5. **Volume math audited for precision** — 16-bit intermediates, u8 coefficients only.
6. **Proof logs audited for privacy** — no wall clock, no frequency, no message text, no document names.

---

## 21. Scan 7 — Exceeded Hypothesis

Assume a rival system sound engine beat Theremin across 10 dimensions:

| Rival Advantage | Why Silk Would Lose | SexOS-Native Fix | Invariant Preserved | Proof Gate |
|----------------|---------------------|------------------|-------------------|------------|
| System sounds always play | No SexAudio → no sound at all | Proof-marker-only mode: when SexAudio unavailable, Theremin logs intents as proof markers without generating audio. | §13.10: Missing SexAudio → safely dropped | T1 |
| Sound is consistent regardless of CPU load | Physical modeling could exceed budget | Bounded polyphony (max 2 voices). Max modes per preset. Tick-based generation with fixed budget. | §13.13: Bounded CPU/memory | T5 |
| No assets needed | No codec decoder = no rich sounds | Physical modeling generates sounds from integer/fixed-point parameters. No codec/asset/waveform dependency. | §13.1: No codec/asset dependency | T6 |
| Sound output is always the same for same event | Floating-point rounding could produce drift | Integer-only DSP: fixed-point arithmetic, integer tables, deterministic envelope/pitch. Same event+preset = same output. | §13.8: Deterministic from parameters | T6 |
| SexAudio downtime doesn't break sound system | Sound system idles while SexAudio unavailable | Theremin operates independently in proof-marker-only mode. When SexAudio available, sends ThereminIntent. No blocking dependency. | §13.3-4: No direct hardware path | T4 |
| No float creep into critical path | Float can't be avoided for pitch/filter | pitch_milli_hz u32 at 1/1000 resolution. Volume uses 16-bit intermediates. No float in synthesis hot path. | §13.28: Integer pitch precision | T5 |
| Privacy-safe proof logs | Sound event timing could leak behavior | Proof logs use shell tick count only. No wall clock, no frequency, no message text, no document names. | §13.27: No wall clock or frequency pattern | T8 |
| Mute/privacy enforced consistently | Critical sound could bypass mute | V1: no bypass permitted. Critical/security sounds respect mute and privacy. Future bypass requires full capability model. | §13.7: No bypass in V1 | T7 |
| Proof markers make failures obvious | Sound failure silently swallowed | Every intent/drop/mute produces proof marker. Drop includes reason string (too_many_voices, policy). | §13.17: Every sound maps to policy decision | T8 |
| Customization is rich but safe | Custom presets could enable float/unbounded resources | All customization (§22) is settings/shell-owned, cannot bypass integer-only DSP, bounded polyphony, or SexAudio routing. | §13.15: Theme changes only affect future sounds | T6 |

## 22. Scan 8 — Customization / User Policy Surface

Customization is settings/shell-owned, validated, reversible, and unable to customize away Theremin's no-float, no-codec, bounded-resource, SexAudio-routed safety.

### Customizable (10 domains)

| Preference | Options | Constraint |
|-----------|---------|------------|
| Per-category volume | u8 0..255 per category | Master volume multiplied: category ≤ master. Cannot exceed hardware-safe max. |
| Theme/preset selection | compiled theme name | Must match compiled preset table. Unknown theme → fallback to default. §13.20: No runtime theme loading. |
| Category enable/disable | on/off per category | Disabled category produces `[theremin.policy.mute]` proof marker, no audio intent. |
| Master volume | u8 0..255 | Applied after category volume. Cannot exceed hardware-safe max. |
| Sound profile | minimal/normal/detailed | Minimal: only accessibility/security/critical. Normal: all enabled categories. Detailed: all including decorative. |
| Preview behavior | on/off (Harp preview of presets) | Preview through SexAudio policy. Harp cannot bypass mixing/ducking/privacy. |
| Proof verbosity | minimum/normal/debug | Cannot suppress required safety markers (`[theremin.intent]`, `[theremin.intent.drop]`, `[theremin.policy.mute]`). |
| Keybindings (future) | scancode+modifiers | After D accessibility + shortcut audit. |
| Event-specific preset override (future) | preset_name per event_type | Override must select from compiled presets only. Unknown preset → fallback to default. |
| Accessibility sound reduction | enabled/disabled | Reduces non-critical sounds. Cannot suppress accessibility/security/critical sounds. |

### Not Customizable (11 hard boundaries)

No float DSP in synthesis hot path. No codec/asset/waveform decoder. No filesystem dependency for boot/error/security sounds. No direct audio hardware access (SexAudio only). No bypass of SexAudio mixer/routing. No bypass of mute/privacy for critical/security sounds in V1. Bounded polyphony (max 2 voices). No dynamic preset/themes loading. No unbounded resonator/modal bank counts. No kernel/PDX ABI edits. sexdisplay ownership (Theremin has no visual output).

### Customization Proof Scenarios

1. Per-category volume=0 for notification → `[theremin.policy.mute]` for notification events. Other categories unaffected.
2. Unknown theme selected → `[theremin.pref.reject]` reason=unknown_theme. Fallback to default theme.
3. Sound profile=minimal → only accessibility/security/critical sounds generate intents. Decorative sounds produce `[theremin.policy.mute]` proof marker only.
4. Master volume applied after category volume → category=200, master=128 → effective volume = (200 × 128) / 255 = 100. Uses 16-bit intermediates.
5. Proof verbosity minimum still emits `[theremin.intent]`, `[theremin.intent.drop]`, `[theremin.policy.mute]` — required safety markers never suppressed.
6. Preview through Harp → `[theremin.intent]` logged with preview=1. SexAudio enforces mixing/ducking.
7. Event-specific preset override selects unknown preset → `[theremin.pref.reject]` reason=unknown_preset. Fallback to default.
8. Keybinding before audit rejected → `[theremin.pref.reject]` reason=no_audit. Planned-only until D accessibility gate.
9. Accessibility sound reduction enabled → reduces non-critical sounds. Accessibility/security/critical sounds at full volume. Verified by proof comparison.
10. Reset-to-safe-default restores canonical behavior → `[theremin.pref.reset]`. All preferences back to compiled defaults.

### Preference Lifecycle

1. **Load** → `[theremin.pref.load]`. 2. **Validate** → `[theremin.pref.validate.ok]` or `.reject`. 3. **Apply** → `[theremin.pref.apply]` (immediate for volume/profile; policy prefs need guard re-validation). 4. **Persist** → blocked until E gates pass (memory-only in V1). 5. **Redact** → `[theremin.pref.redact]` per E8 policy. 6. **Reset** → `[theremin.pref.reset]`.
