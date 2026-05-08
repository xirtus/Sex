#!/usr/bin/env python3
"""Cursor route liveness checker (host-side only).

Proves the input chain is alive end-to-end:
  sexusb → sexinput → silk-shell → sexdisplay (cursor draw)

Usage:
  python3 scripts/check_cursor_route_log.py /tmp/sexos.log
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SEXINPUT_SEND_RE = re.compile(
    r"\[sexinput\.pointer\.send\]\s+class=2\b"
)

SHELL_HID_RE = re.compile(
    r"\[(?:silk-shell\.(?:linen_sync\.input_hid|pointer\.recv|hid\.raw))\]\s+class=2\b"
)

DISPLAY_UPDATE_RE = re.compile(
    r"\[sexdisplay\.cursor\.surface\.update\]\s+n=0\s+x=(\d+)\s+y=(\d+)"
)

DISPLAY_DRAW_RE = re.compile(
    r"\[sexdisplay\.cursor\.draw\]\s+n=0\s+x=(-?\d+)\s+y=(-?\d+)"
)

FATAL_RE = re.compile(
    r"fault\.kill|#PF|#GP|KERNEL PANIC|\bpanic\b"
)

CENTER_X = 640
CENTER_Y = 360


def check_cursor_route(log_path: str) -> int:
    path = Path(log_path)
    if not path.exists():
        print(f"[cursor.route.FAIL] missing=log_file path={log_path}")
        return 1

    text = path.read_text(errors="replace")

    results: dict[str, bool] = {
        "sexinput_send": False,
        "shell_hid": False,
        "display_update": False,
        "display_draw": False,
        "cursor_moved_from_center": False,
        "fatal_fault_or_panic": False,
    }

    for line in text.splitlines():
        if FATAL_RE.search(line):
            results["fatal_fault_or_panic"] = True
        if SEXINPUT_SEND_RE.search(line):
            results["sexinput_send"] = True
        if SHELL_HID_RE.search(line):
            results["shell_hid"] = True
        m = DISPLAY_UPDATE_RE.search(line)
        if m:
            results["display_update"] = True
        m = DISPLAY_DRAW_RE.search(line)
        if m:
            results["display_draw"] = True
            x = int(m.group(1))
            y = int(m.group(2))
            if x != CENTER_X or y != CENTER_Y:
                results["cursor_moved_from_center"] = True

    if results["fatal_fault_or_panic"]:
        print("[cursor.route.FAIL] fatal_fault_or_panic detected")
        return 1

    missing = [k for k, v in results.items()
               if not v and k != "fatal_fault_or_panic"]
    if missing:
        print(f"[cursor.route.FAIL] missing={','.join(missing)}")
        return 1

    print("[cursor.route.PASS] sexinput->silk-shell->sexdisplay moved cursor")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/check_cursor_route_log.py /tmp/sexos.log",
              file=sys.stderr)
        return 2
    return check_cursor_route(sys.argv[1])


if __name__ == "__main__":
    sys.exit(main())
