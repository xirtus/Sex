# RESTORE_BROWSER_SLOT_NET_ROUTE_BASELINE_V1

## Summary
Restored the accepted baseline markers and SLOT_NET route surface after overedit recovery, without reintroducing packed-text paths.

## Changes
- Added `SLOT_NET` constant back to `crates/sex-pdx/src/lib.rs`.
- Restored Kaleidoscope build/boot staging in `Cargo.toml` and `limine.cfg`.
- Restored Kaleidoscope spawn and SLOT_NET grant markers in `kernel/src/init.rs`.
- Restored Kaleidoscope scalar route proof markers in `apps/kaleidoscope/src/main.rs`.

## Markers restored
- `[kernel.spawn.kaleidoscope] id={} domain_id={}`
- `[slot.net.grant.browser.begin] target=kaleidoscope slot=18 pd={}`
- `[slot.net.grant.browser.ok] slot=18 kaleido_pd={}`
- `[browser.slot.net.static_grant.begin]`
- `[browser.slot.net.route.call] status={}`
- `[browser.slot.net.static_grant.proof.done] ok=1 network=0`
