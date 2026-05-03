# COMMON_FAILURES

## 1. Agent assumes POSIX/Linux
Symptom:
- uses std/thread/sleep/filesystem APIs
- assumes normal userspace model

Fix:
- no_std only
- PDX only
- no libc, no threads
- use existing server/app patterns

## 2. Agent reads all interrupts.rs
Symptom:
- context waste
- accidental exception-path edits
- random serial spam in interrupt path

Fix:
- read `docs/INTERRUPTS_QUICKMAP.md` first
- use `rg` landmarks
- open only needed line ranges

## 3. PDX slot/opcode drift
Symptom:
- sender uses wrong slot/opcode
- receiver listens/decodes different constants
- build passes, runtime ignored

Fix:
- constants in `crates/sex-pdx` or model crate
- avoid magic opcodes in producers/consumers
- add startup contract checks/build gates

## 4. Fake security via caller-provided owner
Symptom:
- client passes `owner_pd` and receiver trusts it

Fix:
- trust only kernel/PDX-stamped `caller_pd`
- if unavailable, STOP FIRST

## 5. Cross-PD pointer bug
Symptom:
- raw pointer passed across PD boundary
- receiver dereferences and faults/corrupts

Fix:
- use scalar args only
- use explicit capability/lend protocol where designed

## 6. Renderer ownership violation
Symptom:
- shell/app writes framebuffer directly

Fix:
- sexdisplay is sole framebuffer writer
- shell/apps send protocol/model updates only

## 7. ABI/spec hash mismatch
Symptom:
- build gate fails after sex-pdx/model edits

Fix:
- confirm ABI change is intentional
- update hash only after review
- record reason in handoff
