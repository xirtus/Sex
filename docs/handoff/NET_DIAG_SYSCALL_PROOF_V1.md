# NET_DIAG_SYSCALL_PROOF_V1

## Scope
Applied bounded changes for net diagnostic syscall path only:
- `kernel/src/hal/pci.rs`
- `kernel/src/syscalls/mod.rs`
- `servers/sexnet/src/main.rs`
- `sexos_build_spec.toml` (ABI hash sync required by build gate)

## Implemented
1. Added persisted source scalar in PCI HAL:
- `NET_DIAG_HTTP_SOURCE: AtomicU8`
- store after existing status/bytes persistence
- getter `get_net_diag() -> (u32, u32, u8)`

2. Added syscall dispatch arm in kernel:
- syscall `52` returns packed u64:
  - bits 63:32 status
  - bits 23:16 source
  - bits 15:0 bytes (u16-capped)
- marker: `[net.diag.syscall.reply] ...`

3. Replaced sexnet canned proof text with dynamic runtime buffer:
- static buffer: `PROOF_BUF[32]`, `PROOF_LEN`
- local raw syscall helper in sexnet only (`syscall`, clobber `rcx/r11`)
- dynamic string format: `HTTP <status> rx=<bytes>b <mock|real|unset>`
- markers:
  - `[net.diag.syscall.call] ...`
  - `[sexnet.dynamic_text.set] ...`
- 0x207/0x208 now serve dynamic buffer
- BODY text path unchanged

4. ABI hash sync method (repo convention):
- source: `scripts/entrypoint_build.sh` lines with
  - `expected_abi_hash="$(spec_get abi_version_hash)"`
  - `actual_abi_hash="$({ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } | sha256sum | awk '{print $1}')"`
- computed hash: `a8255e03888113388e3eeffda9ec9a5566a0860fce59ea587d46c19ebe6c5db8`
- updated in `sexos_build_spec.toml`

## Runtime proof attempt (requested lane)
Command used:

```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/net_diag_syscall_proof_v1.log
```

Observed blocker:
- `qemu-system-x86_64: ... Could not open '/dev/net/tun': No such file or directory`

Result classification:
- `SKIP` in this host lane (environment backend missing `/dev/net/tun`), no runtime marker verdict possible here.
