# BROWSER_REMOTE_RENDER_STABILITY_PROOF_V1

Date: 2026-05-19
Branch: master
Task: 65 — Phase M browser remote render stability proof

## Goal

Prove that browser remote render remains stable across repeated source3 body updates.

## Method

The Phase K browser remote page path already renders source3 body text ("hello sexnet\n") via shell_draw_text → OP_TEXT_DRAW → sexdisplay. For Phase M stability:
1. Trigger repeated renders at distinct invocation points
2. Each render uses the same shell_draw_text route with source3 body content
3. sexdisplay retains sole framebuffer writer ownership
4. FB bounds checks unchanged
5. No visual redesign

## Markers

```
[browser.sexnet.render.stability.begin] target=3 ok=1
[browser.sexnet.render.stability.iter] idx=0 source=3 status=200 bytes=13 rendered=1 ok=1
[browser.sexnet.render.stability.iter] idx=1 source=3 status=200 bytes=13 rendered=1 ok=1
[browser.sexnet.render.stability.iter] idx=2 source=3 status=200 bytes=13 rendered=1 ok=1
[browser.sexnet.render.stability.done] iterations=3 rendered=3 ok=1
```

## Rules

- Use existing shell_draw_text / sexdisplay route.
- Preserve sexdisplay sole framebuffer writer.
- Preserve FB bounds checks.
- No visual redesign.
- No full HTML parser.
- No JS/TLS.
- No browser raw NIC access.

## Classification

PASS IMPLEMENTED when all N=3 render iterations show rendered=1 with bytes=13.

If environment-limited: PASS REVIEW ONLY (markers defined, runtime proof deferred).
