# MASTER_RUNTIME_GATE_V1

- date: 2026-05-07T05:51:47+02:00
- git commit: d84f37d
- log_path: /home/xirtus_arch/Documents/microkernel/.gate_master/serial.log
- probe_seconds: 20
- qemu: qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 -serial file:$LOG -display none -no-reboot -no-shutdown

## Gate Results

| Gate | Status |
|------|--------|
| BUILD_GATE | FAIL |
| SPAWN_GATE | PASS |
| CLOCK_GATE | FAIL |
| SCHED_GATE | FAIL |
| FAULT_GATE | PASS |
| SEXFILES_GATE | SKIP |
| **FINAL_SCORE** | **RED_MASTER** |

## Marker Counts

- Spawned PDs: 12
- Clock ticks (silkbar.clock.send): 0
- task.running total: 1
- Fault/panic hits: 0

## PD Spawn Details

- v sexdisplay: spawned
- v sexdrive: spawned
- v silk-shell: spawned
- v sexinput: spawned
- v silkbar: spawned
- v linen: spawned

- sexfiles: SEXFILES_GATE=SKIP ready=0 kspawn=1

- sexdisplay (PD 1): task.running 1x
- sexdrive (PD 2): task.running 0x
- silk-shell (PD 3): task.running 0x
- sexinput (PD 4): task.running 0x
- silkbar (PD 6): task.running 0x
- linen (PD 7): task.running 0x

- sexfiles (PD 11): task.running 0x

## Clock Liveness

- silkbar.clock.send ticks: 0
- Minimum required: 2

## Expected Pass Criteria

1. **BUILD_GATE**: ISO builds without errors (or --skip-build with existing ISO)
2. **SPAWN_GATE**: All 6 PDs have `v Spawned PD` markers
3. **CLOCK_GATE**: `[silkbar.clock.send]` appears at least twice
4. **SCHED_GATE**: Each PD has at least one `task.running` entry
5. **FAULT_GATE**: No `panic`, `fault.kill`, `#PF`, `#GP`, `FATAL` markers

## Fail Criteria

- Any gate above is FAIL
- QEMU fails to boot within probe window
- ISO missing when --skip-build used

## Run Command

```
./scripts/master_runtime_gate.sh
```

### Variants

```
# Skip rebuild (use existing ISO)
./scripts/master_runtime_gate.sh --skip-build

# Custom probe duration (e.g., 30 seconds)
./scripts/master_runtime_gate.sh --probe 30

# Keep serial log after run
./scripts/master_runtime_gate.sh --keep-log
```

## Known Notes

- sexusb (PD 5) is also present and running but excluded from the 6-PD gate requirement.
- purple-scanout module is loaded but is cosmetic/diagnostic only.
- QEMU is run headless (-display none); no window appears unless --display is overridden.
- Serial log is captured to /home/xirtus_arch/Documents/microkernel/.gate_master/serial.log and preserved only with --keep-log.
