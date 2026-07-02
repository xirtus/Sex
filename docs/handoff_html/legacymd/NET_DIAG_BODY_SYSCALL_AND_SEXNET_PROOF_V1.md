# NET_DIAG_BODY_SYSCALL_AND_SEXNET_PROOF_V1

Date: 2026-05-18
Backup: /tmp/microkernel-backup-20260518-122443.tar.gz
Proof log: /tmp/net_diag_body_syscall_and_sexnet.log

## Scope completed
- `kernel/src/syscalls/mod.rs`
  - Syscall `52` now dispatches by `rdi` selector:
    - `0`: existing packed status/source/bytes path with existing marker
    - `1`: body length via `get_net_diag_body_len()` + marker
    - `2..=9`: body chunks via `get_net_diag_body_chunk(idx)` + marker
    - default: `u64::MAX`
- `servers/sexnet/src/main.rs`
  - `sys_net_diag(selector: u64)` now sets `rdi`
  - startup uses `sys_net_diag(0)` for legacy status
  - when `source == 2`, fetches len via selector `1`, chunks via selectors `2+ci`, sanitizes CR/LF to spaces, stores into local static `BODY_BUF`, sets `BODY_LEN`, emits markers
  - opcodes `0x209/0x20A` now serve dynamic body when `BODY_LEN > 0`, fallback to `BODY_TEXT` otherwise
- `sexos_build_spec.toml`
  - ABI hash updated per repo gate formula

## ABI hash method/result
- Method (from `scripts/entrypoint_build.sh`):
  - `({ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } | sha256sum | awk '{print $1}')`
- Previous spec hash:
  - `a8255e03888113388e3eeffda9ec9a5566a0860fce59ea587d46c19ebe6c5db8`
- New computed hash:
  - `e54149fe45653dbe6adc064b3056dc73711958a1d31f5b0ee0487862df9007d6`
- `sexos_build_spec.toml` now matches new computed hash.

## Proof execution result
Command run:
- `QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 ENABLE_QEMU_USERNET_E1000=1 ./scripts/run_daily_driver_proof.sh /tmp/net_diag_body_syscall_and_sexnet.log`

Observed blocker:
- `qemu-system-x86_64: -netdev tap,id=net0,ifname=tap0,script=no,downscript=no: Could not open '/dev/net/tun': No such file or directory`

Impact:
- Build phase passed.
- Boot/runtime network proof did not run; log has no net-diag/body/browser markers.

## Status
- Runtime verdict: SKIP (environment TAP blocker)
- Not a code STOP condition from mission constraints.
