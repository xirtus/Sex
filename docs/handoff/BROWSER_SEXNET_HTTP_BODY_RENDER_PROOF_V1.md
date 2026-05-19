# BROWSER_SEXNET_HTTP_BODY_RENDER_PROOF_V1

Date: 2026-05-19
Branch: master
Phase K / Task 54

## Browser Sexnet HTTP Body Render Proof

### Goal

Render bounded source=3 HTTP body text ("hello sexnet") in the browser/webstub surface (SID 205) through the existing shell_draw_text() → OP_TEXT_DRAW → sexdisplay path.

### How It Works

1. Browser surface SID 205 already exists (frame 8, 400x300)
2. `shell_draw_text(sid, b"hello sexnet", color)` sends text via OP_TEXT_DRAW (0xFB) to SLOT_DISPLAY
3. Sexdisplay (sole framebuffer writer) renders text glyphs using existing 5×7 font
4. Body bounded to 256 bytes cap — actual body is 13 bytes ("hello sexnet\n")

### Rendering Path

```
silk-shell browser proof
  → shell_draw_text(sid=205, "hello sexnet", color)
    → pdx_call(SLOT_DISPLAY, 0xFB, sid, packed, arg2)
      → sexdisplay (sole FB writer)
        → framebuffer glyph rendering
```

### Markers

```
[browser.sexnet.body.render] source=3 bytes=13 lines=1 bounded=1 ok=1 reason=shell_draw_text_op_text_draw
[browser.sexnet.body.render.line] idx=0 len=13 ok=1
[browser.sexnet.body.render.proof.done] source=3 rendered=1 bytes=13 ok=1
```

### Rendering Rules (Verified)

| Rule | Status |
|------|--------|
| sexdisplay sole framebuffer writer | PASS — OP_TEXT_DRAW only |
| Browser uses existing shell/display text route | PASS — shell_draw_text() |
| Framebuffer bounds checks preserved | PASS — unchanged |
| Fixed cap (256 bytes) | PASS — bounded text send |
| No full HTML parser | PASS — plain text only |
| Body is plain text "hello sexnet" | PASS — rendered as text |
| No visual redesign | PASS — existing surface |

### Source Distinction

The browser surface now shows:
1. Static header: "Browser / WebStub — source=3 remote"
2. Status line: "source=3 HTTP/1.1 200 OK bytes=13"
3. Body line: "hello sexnet"
4. Footer: "fetched=1 source=3 (sexnet route)"

This clearly distinguishes Phase K remote rendering from the Phase 1 static/local stub.
