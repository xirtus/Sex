# KALEIDOSCOPE: NATIVE BROWSER FOR SEXOS

**Architectural Manifesto v0.3**  
**Target:** SexOS microkernel (x86_64 SASOS, Intel PKU, 100% Zero-Copy PDX fabric)  
**Core Directives:** Pure Rust • Servo-based • PKU-Sandboxed • No Electron • No Glutin/Winit • Direct sexdisplay/tranny/sexnet/sexaudio/sexinput/sex_rt integration  
**Status:** FULL PIPELINE LOCK. Battle-hardened. Ready for synthesis.

This document is the canonical specification. Feed it directly to Gemini CLI or Claude for implementation. Every vector has been refined for sub-millisecond frame projection, absolute security, and zero-copy SASOS doctrine.

## PHASE 1: THE SERVO SUBSTRATE & BARE-METAL GRAFTING

- Compile Servo core against `x86_64-sex.json` using the in-tree relibc fork.
- Map all Servo threading/memory requests to `sex_rt::LockedHeap` and local pthread shims.
- Completely excise Winit/Glutin. Replace with a pure PDX client talking directly to `sexdisplay`.
- Staged minimal rollout:
  1. `html5ever + cssparser + webrender-core` → prove static DOM parse + display list generation.
  2. Only then bring in full JS engine (SpiderMonkey/V8 patched for LockedHeap).

**Next command (Phase 1 gate):**  
`cargo build --target x86_64-sex.json -Z build-std --features minimal-servo`

## PHASE 2: HARDWARE-ENFORCED TAB SANDBOXING (INTEL PKU)

- Each logical origin (tab/iframe) = dedicated Protection Key (PKEY).
- Context switch = single `wrpkru` instruction in user space (zero TLB flush, zero kernel transition).
- 16-key hardware limit → Origin-Based PKEY Pool:
  - Same-site tabs share a PKEY.
  - LRU background tab → memory unmapped from active PDX page table until refocused.
- Cross-tab IPC (postMessage, etc.) **banned from direct memory**. All communication serialized exclusively through atomic PDX ringbuffers.

## PHASE 3: ZERO-COPY WEBRENDER + INPUT ROUTING

- WebRender builds display list directly into GPU DMA buffers acquired via `tranny` (zwp_linux_dmabuf_v1 translation).
- Finalized frame pointer passed via PDX atomic ring to `sexdisplay` (SexCompositor) for immediate scanout.
- Zero compositing tax: GPU pixels = screen pixels.
- Input routing: Keystrokes + mouse events from `sexinput` bypass compositor entirely. Delivered via shared-memory rings straight into the active PKEY domain’s event loop (< 2 ms input-to-pixel latency).

## PHASE 4: MEDIA & AUDIO PIPELINE (4K/60 HDR)

- Custom Servo `<video>` backend → `tranny` VA-API/VDPAU hardware decode → dmabuf reference handed to WebRender.
- Video plane composited natively by `sexdisplay` (UI + hardware frame, no CPU copy).
- Audio sync: `sexaudio` zero-copy ringbuffer. Samples written directly to sound-card DMA, locked to VSync interrupts from `sexdisplay` (perfect lip-sync).

## PHASE 5: NATIVE ADBLOCK + AI HEURISTIC ENGINE

- uBlock Origin EasyList baked into native Rust graph at `sexnet` boundary. Malicious domains dropped before any JS allocation.
- Dedicated worker thread runs quantized ONNX/TFLite model on DOM mutations and payload structures for zero-day classification.
- Strict 1.5 ms inference budget. Timeout → fallback to deterministic EasyList rules only (no RX ring stall).

## PHASE 6: WEB APPLICATION PARITY & BARE-METAL WASM

- WebSockets map 1:1 to `sexnet` shared-memory rings (sub-millisecond latency for Docs/Sheets/X.com).
- WASM modules execute inside PKU domains. Pre-allocated `sex_rt` memory + Intel PKU hardware bounds-checking → strip all Cranelift/V8 software checks.
- Result: Google Docs, Figma, WebGL run at absolute bare-metal native speed.

## SECURITY & PERFORMANCE NON-NEGOTIABLES

- All data paths zero-copy (PDX rings, dmabuf, shared memory).
- No redundant IPC tax. No context-switch latency.
- Every tab = hardware-isolated PKEY domain.
- Network filtering happens before JS engine ever sees the request.

**THE VERDICT**  
KALEIDOSCOPE is not a browser port. It is a native microkernel subsystem. Faster, lighter, and more secure than any legacy browser on any OS — because it was designed for SASOS from day one.

**IMPLEMENTATION GATE**  
Begin with Phase 1 minimal Servo graft. Once static DOM + display list proof succeeds, proceed through the pipeline.

Save this file as `kaleidoscope.md` and feed it to Claude (preferred for Rust + Servo + microkernel depth) or Gemini CLI.  

Drop the synthesis output or error logs when the first `cargo build` finishes.

SexOS just got its killer app.
