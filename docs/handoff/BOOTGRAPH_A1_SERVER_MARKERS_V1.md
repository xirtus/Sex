# BOOTGRAPH_A1_SERVER_MARKERS_V1

## Scope
Marker-only updates in server/app entry paths. No kernel, sex-pdx, ABI, or behavior changes.

## Canonical markers added/fixed
- sexdisplay: `[sexdisplay.init.start]`, `[sexdisplay.ready]`
- sexdrive: `[sexdrive.init.start]`, `[sexdrive.ready]`
- silkshell: `[silkshell.init.start]`, `[silkshell.ready]`
- sexinput: `[sexinput.init.start]`, `[sexinput.ready]`
- sexusb: `[sexusb.init.start]`, `[sexusb.ready]`, milestone `[sexusb.xhci.ring]`
- silkbar: `[silkbar.init.start]`, `[silkbar.ready]`
- linen: `[linen.init.start]`, `[linen.ready]`, milestone `[linen.session.init]`
- sexstore: `[sexstore.init.start]`, `[sexstore.ready]`
- quil: `[quil.init.start]`, `[quil.ready]`, milestone `[quil.diskfs.mount]`
- sexbell: `[sexbell.init.start]`, `[sexbell.ready]`
- sexfiles: `[sexfiles.init.start]`, `[sexfiles.ready]`, milestone `[sexfiles.ramfs.mount]`
- spindle: `[spindle.init.start]`, `[spindle.ready]`

## Placement notes
- `init.start`: top of `_start()`.
- `ready`: immediately before first blocking main loop / receive path.
- No logic move, no conditional-only ready marker.

## Verification
- `./scripts/entrypoint_build.sh` passed.
- Runtime grep on fresh `/tmp/sexos.log` shows all touched `init.start` and `ready` markers and no `fault.kill/#PF/#GP/panic` lines in that grep output.
