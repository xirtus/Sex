# FOUNDATION MPK/PKU — Chunk 2: PKEY Map

Canonical assignment (13 of 16 PKEYs used):

| PKEY | Assignment | Type | Purpose |
|------|-----------|------|---------|
| 0 | Kernel | Static | All kernel pages (USER_ACCESSIBLE=0) |
| 1 | sexdisplay | Domain (id=1) | Framebuffer + private |
| 2 | sexdrive | Domain (id=2) | Private |
| 3 | silk-shell | Domain (id=3) | Private |
| 4 | sexinput | Domain (id=4) | Private |
| 5 | sexusb | Domain (id=5) | Private |
| 6 | silkbar | Domain (id=6) | Private |
| 7 | linen | Domain (id=7) | Private |
| 8 | sexstore | Domain (id=8) | Private |
| 9 | quil | Domain (id=9) | Private |
| 10 | sexbell | Domain (id=10) | Private |
| 11–13 | *Free* | — | Future domains |
| 14 | SHARED | Fixed | IPC buffers (sexdrive:RW, sexdisplay:RO, others:NoAccess) |
| 15 | UNTRUSTED | Fixed | Syscall return buffer (kernel-owned, user-denied by PKRU) |

**PKRU derivation:** `PkruValue::for_domain(pkey)` in `capability.rs:186-217`.
- Always opens PKEY 0 (kernel)
- Opens own PKEY as RW
- PKEY 14: sexdrive=RW, sexdisplay=RO, others=NoAccess
- All other PKEYs: NoAccess
