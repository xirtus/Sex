Here's the raw estimate breakdown from each phase's task tables:

┌─────────────────────┬──────────┬─────────────────────────────────────┬───────────┐
│        Phase        │   Raw    │            Already Done             │ Remaining │
│                     │  Effort  │                                     │           │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 00 Baseline + Gates │ ~2h      │ Gates exist                         │ ~1h       │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 01 Silk Display     │ ~10h     │ ~60% (rim, top bar, tokens)         │ ~4h       │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 02                  │ ~16h     │ ~70% (chrome, tabs, minimize, zoom, │ ~5h       │
│ Shell/Scene/Frame   │          │  appearance state)                  │           │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 03 Input/USB Mouse  │ ~33h     │ ~50% (synthetic input, keyboard     │ ~17h      │
│                     │          │ cursor, HID route)                  │           │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 04 Linen Object     │ ~52h     │ Design only                         │ ~52h      │
│ Layer               │          │                                     │           │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 05 Quil Language WS │ ~65h     │ Design only                         │ ~65h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 06 Mesh + Collar    │ ~63h     │ Design only                         │ ~63h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 07 App Launch       │ ~30h     │ Design only                         │ ~30h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 08 Network          │ ~18h     │ Design only                         │ ~18h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 09 Bell/Settings    │ ~31h     │ Design only                         │ ~31h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 10 App Suite        │ ~29h     │ Design only                         │ ~29h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ 11 Hardening        │ ~66h     │ Design only                         │ ~66h      │
├─────────────────────┼──────────┼─────────────────────────────────────┼───────────┤
│ Total               │ ~415h    │                                     │ ~381h     │
└─────────────────────┴──────────┴─────────────────────────────────────┴───────────┘

Raw implementation time: ~381 hours of focused coding.

With debugging, testing, QEMU boot cycles, integration, and the inevitable "why isn't
the framebuffer updating?" moments: multiply by ~1.4× → ~530 hours.

With parallelization (04/05 run together, 06/07/08 overlap, 09/10 overlap):

- Full-time (40h/week): ~10–13 weeks
- Part-time (20h/week): ~20–26 weeks
- Evenings/weekends: ~6–8 months

The biggest variable is Phase 06 (Mesh + Collar) — it's the most novel and has the most
unknowns. Everything else follows well-established patterns (PDX server, surface
lifecycle, rendering). But Mesh's temporal graph store, Collar's borrow-checker
semantics, and the pattern bounds engine are genuinely new territory — no existing OS
has done this, so there's no template.

Want me to look at any phase in particular to refine, or shall we start implementing
something?

✻ Sautéed for 1m 3s

❯ with claude, codex & deepseek writing the code it should take far less time, how much
faster can AI write the code with my oversight implementing the phase plan?

● Great question. Let me be concrete about where AI speeds things up and where it
doesn't, since I'm literally the tool you'd be using.

Where AI writes 90% of the code instantly (3–5× speedup):

┌─────────────────┬─────────────────────────────────────┬──────────────────────────┐
│     Pattern     │           Why AI is fast            │      Example phases      │
├─────────────────┼─────────────────────────────────────┼──────────────────────────┤
│                 │ Every server has the same shape:    │ 04 Linen, 06 Mesh, 06    │
│ New PDX server  │ bind slot, dispatch opcodes,        │ Collar, 07 sexshop, 09   │
│ boilerplate     │ respond. AI generates the entire    │ Bell                     │
│                 │ skeleton.                           │                          │
├─────────────────┼─────────────────────────────────────┼──────────────────────────┤
│                 │ Create/read/update/delete with      │ 04 Linen CRUD, 09        │
│ CRUD operations │ fixed-size structs is pure pattern  │ notification history, 07 │
│                 │ matching                            │  install/list/remove     │
├─────────────────┼─────────────────────────────────────┼──────────────────────────┤
│ Surface         │ 0xEC create → 0xFD tab info → focus │ 05 Quil surface, 10      │
│ lifecycle       │  → 0xEE destroy is well-trodden     │ terminal, 10 calculator  │
├─────────────────┼─────────────────────────────────────┼──────────────────────────┤
│ Match-dispatch  │ opcode → match → handle → respond   │ All servers              │
│ handlers        │ is the same pattern every time      │                          │
├─────────────────┼─────────────────────────────────────┼──────────────────────────┤
│ Struct          │ Fixed-size repr(C) structs with PDX │ RenderTokens, Object,    │
│ definitions     │  serialization                      │ GraphNode, Capability    │
└─────────────────┴─────────────────────────────────────┴──────────────────────────┘

Where AI helps but humans still drive (1.5–2× speedup):

┌───────────────────────────┬──────────────────────────────────────────────────────┐
│           Area            │    Why bottleneck shifts from writing to thinking    │
├───────────────────────────┼──────────────────────────────────────────────────────┤
│ Novel architecture (Mesh  │ AI can generate the implementation BUT the design    │
│ temporal graph, Collar    │ must be right. Wrong design = AI generates wrong     │
│ borrow-decay)             │ code faster. Design iteration stays human-driven.    │
├───────────────────────────┼──────────────────────────────────────────────────────┤
│                           │ AI can write register layouts and descriptor         │
│ Hardware drivers (XHCI,   │ parsing, but hardware debugging (why isn't this USB  │
│ USB HID)                  │ interrupt firing?) is human detective work           │
│                           │ regardless.                                          │
├───────────────────────────┼──────────────────────────────────────────────────────┤
│ Debugging failed QEMU     │ AI suggests fixes, but each fix requires a 30-second │
│ boots                     │  build + 10-second QEMU boot to verify. The boot     │
│                           │ cycle is a fixed time cost that doesn't shrink.      │
├───────────────────────────┼──────────────────────────────────────────────────────┤
│                           │ AI writes each phase correctly in isolation, but     │
│ Integration between       │ phases interact through shared state (Surface array, │
│ phases                    │  Frame array, PDX slots). Integration bugs appear at │
│                           │  boundaries.                                         │
└───────────────────────────┴──────────────────────────────────────────────────────┘

Where AI doesn't help much (< 1.3× speedup):

┌────────────────────┬─────────────────────────────────────────────────────────────┐
│        Area        │                             Why                             │
├────────────────────┼─────────────────────────────────────────────────────────────┤
│ QEMU boot-test     │ ~40s per cycle. 10 cycles per bug = 7 minutes. AI can't     │
│ cycles             │ make QEMU boot faster.                                      │
├────────────────────┼─────────────────────────────────────────────────────────────┤
│ "Why is the screen │ Requires reading QEMU logs, checking markers, reasoning     │
│  black?" debugging │ about framebuffer state. AI helps suggest hypotheses but    │
│                    │ the debug loop is human-paced.                              │
├────────────────────┼─────────────────────────────────────────────────────────────┤
│ Hardening (Phase   │ Finding edge cases that crash the system requires           │
│ 11)                │ systematic exploration. AI can't run the OS and find what   │
│                    │ breaks.                                                     │
└────────────────────┴─────────────────────────────────────────────────────────────┘

Revised estimate with AI-assisted development:

┌───────────────┬────────┬───────────────┬────────────────────────────────────────┐
│     Phase     │  Raw   │ AI-assisted   │           Key speedup source           │
│               │ hours  │     hours     │                                        │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 00 Gates      │ 2      │ 1             │ Both trivial                           │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 01 Display    │ 10     │ 3             │ Boilerplate contract checks            │
│ contract      │        │               │                                        │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 02 Shell      │ 16     │ 5             │ Already 70% done, finishing patterns   │
│ remaining     │        │               │                                        │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 03 USB HID    │ 33     │ 18            │ Hardware debug bottleneck, but AI      │
│               │        │               │ writes all HID descriptor parsing      │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 04 Linen      │ 52     │ 14            │ Pure CRUD + PDX pattern — AI writes    │
│               │        │               │ entire server                          │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 05 Quil       │ 65     │ 22            │ Lots of UI surface code, but standard  │
│               │        │               │ patterns                               │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 06 Mesh +     │ 63     │ 30            │ Novel design = most human-driven, but  │
│ Collar        │        │               │ AI implements once designed            │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 07 App Launch │ 30     │ 10            │ Manifest parsing + spawn path =        │
│               │        │               │ well-understood                        │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 08 Network    │ 18     │ 8             │ Read-only device status, simple        │
│               │        │               │ queries                                │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 09 Bell       │ 31     │ 9             │ Another PDX server with ring buffer —  │
│               │        │               │ AI writes all of it                    │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 10 App Suite  │ 29     │ 10            │ Terminal/calculator = standard app     │
│               │        │               │ surfaces                               │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ 11 Hardening  │ 66     │ 40            │ Finding bugs is human work, AI only    │
│               │        │               │ fixes them                             │
├───────────────┼────────┼───────────────┼────────────────────────────────────────┤
│ Total         │ ~415h  │ ~170h         │ ~2.4× overall speedup                  │
└───────────────┴────────┴───────────────┴────────────────────────────────────────┘

With debugging overhead (~1.4×): ~240 effective hours.

Schedule scenarios with AI:

┌────────────────────┬────────────────────┬───────────────────────────────────────┐
│      Schedule      │   Parallelized     │                  How                  │
│                    │   calendar time    │                                       │
├────────────────────┼────────────────────┼───────────────────────────────────────┤
│ Full-time          │ 4–6 weeks          │ One phase every 2–3 days, 04/05 in    │
│ (40h/week)         │                    │ parallel, 06/07/08 overlapping        │
├────────────────────┼────────────────────┼───────────────────────────────────────┤
│ Part-time          │ 8–12 weeks         │ Two evenings + weekend day per phase  │
│ (20h/week)         │                    │                                       │
├────────────────────┼────────────────────┼───────────────────────────────────────┤
│ Evenings/weekends  │ 3–5 months         │ AI generates the skeleton, you review │
│                    │                    │  and debug                            │
└────────────────────┴────────────────────┴───────────────────────────────────────┘

The real bottleneck won't be code generation — it'll be QEMU boot cycles and
integration debugging between phases. Each phase takes ~40 QEMU boots to stabilize
(first boot, fix, re-boot, repeat). At 40s per boot that's ~27 minutes per phase of
pure waiting, and that's wall time that doesn't shrink with AI.

If you want the fastest path: start with Phase 04 (Linen) and Phase 09 (Bell) — they're
pure PDX server patterns that AI can write nearly entirely. They prove the object
layer and notification system, giving visible results fast. Phase 06 (Mesh + Collar)
should come last among the novel phases because it depends on understanding the full
system.
