# HOSTFWD_SYNTAX_MATRIX_PROBE_V1

Date: 2026-05-17
Status: Executed; no variant reached boot

## Objective
Try small bounded QEMU hostfwd syntax variants until one boots, without kernel changes, and verify no duplicate netdev/usernet construction.

## Pre-check: no duplicate netdev/usernet construction

From `scripts/run_daily_driver_proof.sh`:

- Single netdev value is constructed (`user,id=net0[,hostfwd=...]` or `tap,id=net0,...`).
- Single device attach uses `netdev=net0`.
- No second usernet/netdev path exists in the boot command construction block.

## Probe matrix (sequential)

Command form used for each variant:

```bash
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  QEMU_USERNET_HOSTFWD='<variant>' \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_hostfwd_syntax_matrix_v1_N.log
```

### Variant 1

- input: `tcp::18080-:18080`
- marker: `[qemu.net.config] ... hostfwd=tcp::18080-:18080 ...`
- result: pre-boot fail
- stderr: `Could not set up host forwarding rule 'tcp::18080-:18080'`

### Variant 2

- input: `tcp:127.0.0.1:18080-:18080`
- marker: `[qemu.net.config] ... hostfwd=tcp:127.0.0.1:18080-:18080 ...`
- result: pre-boot fail
- stderr: `Could not set up host forwarding rule 'tcp:127.0.0.1:18080-:18080'`

### Variant 3

- input: `hostfwd=tcp::18080-:80`
- marker: `[qemu.net.config] ... hostfwd=hostfwd=tcp::18080-:80 ...`
- result: pre-boot fail
- stderr: `Invalid host forwarding rule 'hostfwd=tcp::18080-:80' (Bad protocol name)`

### Variant 4

- input: `hostfwd=tcp:127.0.0.1:18080-10.0.2.15:80`
- marker: `[qemu.net.config] ... hostfwd=hostfwd=tcp:127.0.0.1:18080-10.0.2.15:80 ...`
- result: pre-boot fail
- stderr: `Invalid host forwarding rule 'hostfwd=tcp:127.0.0.1:18080-10.0.2.15:80' (Bad protocol name)`

## Outcome

- No tested variant produced a booting QEMU session.
- Variants 3/4 are syntactically invalid for this env var path (they include `hostfwd=` prefix inside the hostfwd value).
- Variants 1/2 are syntactically accepted but still fail hostfwd setup in this environment.

## Artifacts

- Summary: `/tmp/hostfwd_syntax_matrix_probe_v1_summary.txt`
- Per-variant logs/outs:
  - `/tmp/sexos_hostfwd_syntax_matrix_v1_1.log`, `/tmp/sexos_hostfwd_syntax_matrix_v1_1.out`
  - `/tmp/sexos_hostfwd_syntax_matrix_v1_2.log`, `/tmp/sexos_hostfwd_syntax_matrix_v1_2.out`
  - `/tmp/sexos_hostfwd_syntax_matrix_v1_3.log`, `/tmp/sexos_hostfwd_syntax_matrix_v1_3.out`
  - `/tmp/sexos_hostfwd_syntax_matrix_v1_4.log`, `/tmp/sexos_hostfwd_syntax_matrix_v1_4.out`
