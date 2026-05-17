# BROWSER_INPUT_WINDOWING_SPRINT_EXECUTION_V1

Date: 2026-05-17
Status: Executed in current workspace/runtime lane

## Objective
Run the browser/input/windowing sprint lane safely and sequentially after hostfwd/tap env-fix attempts.

## Command

```bash
./scripts/entrypoint_build.sh
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_browser_input_windowing_sprint_v1.log
```

Artifacts:

- Build output: `/tmp/browser_input_windowing_sprint_v1_build.out`
- Runner output: `/tmp/sexos_browser_input_windowing_sprint_v1.out`
- Runtime log: `/tmp/sexos_browser_input_windowing_sprint_v1.log`

## Result

- Build: PASS
- Runtime gate profile: **PASS (249 gates, 0 fail, 12 skip, 0 faults)**

## Network/browser lane evidence

- `[runtime.smoke.real.network.pipeline.v1] mode=mock backend=user tcp_env_limited=1 syn_tx=1 synack=0 rst=0 mock_mode=1 fetched=1 status=200 ... ok=1`
- `[daily.driver.network.baseline.freeze.v1] ... frozen=1 ... fetched=1 status=200 ... ok=1`
- `[network.sprint.final.runtime.smoke.v1] ... fetched=1 status=200 ... ok=1`
- `[network.sprint.handoff.freeze.v1] ... done=1 ... fetched=1 status=200 ... ok=1`
- `[qemu.slirp.tcp.limit.freeze] ... environment_limited=1 ok=1`

Interpretation:
- Frozen-live TCP truth is unchanged in this host lane (`synack_seen=0`, `rst_seen=0`, no final ACK, no real HTTP send).
- Mock browser fetch/render lane remains healthy and fully gated.

## Input/windowing lane evidence

- `[window.workflow.proof.done] ok=1 passed=6 failed=1`
  - expected unsupported step:
    - `[window.workflow.step] action=close_disposable ... ok=0 reason=unsupported_no_safe_disposable_surface`
- `[shell.keyboard.window.proof.done] ok=1`
- Browser/window UX proof markers present:
  - URL/history/bookmark/tab/action/dashboard/find/reader/save/export proof-done markers
  - all logged with `ok=1` in bounded non-network or mock-network lane

## Safety/classification

- No kernel/ABI scope change in this execution lane.
- No claim of real network handshake success.
- Browser/input/windowing sprint is validated in the current bounded runtime profile.
