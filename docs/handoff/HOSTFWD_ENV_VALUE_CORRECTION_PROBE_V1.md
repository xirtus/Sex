# HOSTFWD_ENV_VALUE_CORRECTION_PROBE_V1

Date: 2026-05-17
Verdict: PASS DIAGNOSTIC (all bounded variants failed pre-boot)

## Scope Guard

- No kernel edits performed.

## 1) Script construction check

Confirmed `scripts/run_daily_driver_proof.sh` constructs exactly one netdev path for user backend:

- `-netdev user,id=net0,hostfwd=${QEMU_USERNET_HOSTFWD}` when hostfwd env is set
- otherwise `-netdev user,id=net0`
- and exactly one matching `-device ... netdev=net0`

This satisfies the “no duplicate netdev/usernet construction” requirement.

## 2) Env value rule

Probe used env values **without** `hostfwd=` prefix.

## 3) Port 18080 occupancy check

Command:

```bash
ss -ltn '( sport = :18080 )'
```

Result in this environment:

- `Cannot open netlink socket: Operation not permitted`

Occupancy could not be authoritatively determined due to host permission boundary.

## 4-7) Variant probe sequence (stop on first boot)

Runner used:

```bash
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
QEMU_USERNET_HOSTFWD='<value>' ./scripts/run_daily_driver_proof.sh <log>
```

### Variant 1

- value: `tcp::18080-:80`
- marker: `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=tcp::18080-:80 tap_if=tap0`
- QEMU error:
  - `qemu-system-x86_64: -netdev user,id=net0,hostfwd=tcp::18080-:80: Could not set up host forwarding rule 'tcp::18080-:80'`
- status: fail_preboot

### Variant 2

- value: `tcp::18081-:80`
- marker: `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=tcp::18081-:80 tap_if=tap0`
- QEMU error:
  - `qemu-system-x86_64: -netdev user,id=net0,hostfwd=tcp::18081-:80: Could not set up host forwarding rule 'tcp::18081-:80'`
- status: fail_preboot

### Variant 3

- value: `tcp:127.0.0.1:18080-10.0.2.15:80`
- marker: `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=tcp:127.0.0.1:18080-10.0.2.15:80 tap_if=tap0`
- QEMU error:
  - `qemu-system-x86_64: -netdev user,id=net0,hostfwd=tcp:127.0.0.1:18080-10.0.2.15:80: Could not set up host forwarding rule 'tcp:127.0.0.1:18080-10.0.2.15:80'`
- status: fail_preboot

### Variant 4

- value: `tcp:127.0.0.1:18081-10.0.2.15:80`
- marker: `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=tcp:127.0.0.1:18081-10.0.2.15:80 tap_if=tap0`
- QEMU error:
  - `qemu-system-x86_64: -netdev user,id=net0,hostfwd=tcp:127.0.0.1:18081-10.0.2.15:80: Could not set up host forwarding rule 'tcp:127.0.0.1:18081-10.0.2.15:80'`
- status: fail_preboot

## 8) Stop condition

- No variant booted, so stop-on-first-boot condition was never reached.

## Artifacts

- `/tmp/hostfwd_env_value_correction_probe_v1_summary.txt`
- `/tmp/sexos_hostfwd_env_value_correction_v1_1.log`, `/tmp/sexos_hostfwd_env_value_correction_v1_1.out`
- `/tmp/sexos_hostfwd_env_value_correction_v1_2.log`, `/tmp/sexos_hostfwd_env_value_correction_v1_2.out`
- `/tmp/sexos_hostfwd_env_value_correction_v1_3.log`, `/tmp/sexos_hostfwd_env_value_correction_v1_3.out`
- `/tmp/sexos_hostfwd_env_value_correction_v1_4.log`, `/tmp/sexos_hostfwd_env_value_correction_v1_4.out`
