#!/usr/bin/env python3
"""
QEMU QMP mouse injection script (dev infra only).

Connects to /tmp/sexos-qmp.sock and sends mouse events via QMP
input-send-event (broadcast) or HMP human-monitor-command.

KNOWN LIMITATION (QEMU 11.0, usb-mouse device):
QMP input-send-event returns success but events do NOT reach the
emulated usb-mouse device. They are consumed by the PS/2 display layer.
No device name was found that routes to usb-mouse (all common names fail).
This means mouse injection cannot replace real mouse input for this setup.
Use usb-tablet (absolute HID) instead.

Usage:
    # Start QEMU with injection enabled:
    SEXOS_QEMU_INPUT_INJECT=1 ./dev.sh run

    # In another terminal after desktop appears:
    python3 scripts/qemu_mouse_inject.py

NO Rust/kernel/ABI/cap changes. Dev infra only.
"""

import argparse
import json
import socket
import sys
import time

QMP_SOCKET = "/tmp/sexos-qmp.sock"


class QMPClient:
    """Minimal QMP client over Unix socket."""

    def __init__(self, sockpath: str):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(10.0)
        self.sock.connect(sockpath)
        self.fp = self.sock.makefile("rw", buffering=1)
        # Consume QMP greeting
        self._read_response()
        # Capabilities handshake (_cmd sends + reads response)
        resp = self._cmd({"execute": "qmp_capabilities"})
        if "error" in resp:
            print(f"QMP capabilities error: {resp}", file=sys.stderr)
            sys.exit(1)

    def _read_response(self) -> dict:
        """Read one JSON object from QMP socket (newline-delimited)."""
        line = self.fp.readline()
        if not line:
            raise ConnectionError("QMP socket closed")
        return json.loads(line.strip())

    def _cmd(self, cmd: dict) -> dict:
        """Send a QMP command and return the response."""
        self.fp.write(json.dumps(cmd) + "\n")
        self.fp.flush()
        return self._read_response()

    def send_events(self, events: list, device: str = ""):
        """
        Send input events via QMP input-send-event.
        If device is empty string, broadcasts to all input handlers.
        """
        args = {"events": events}
        if device:
            args["device"] = device
        return self._cmd({
            "execute": "input-send-event",
            "arguments": args
        })

    def mouse_move_rel(self, dx: int, dy: int, device: str = ""):
        """Send relative mouse movement."""
        ev = [
            {"type": "rel", "data": {"axis": "x", "value": dx}},
            {"type": "rel", "data": {"axis": "y", "value": dy}},
        ]
        result = self.send_events(ev, device)
        status = "ok" if "return" in result else str(result.get("error", ""))
        print(f"  mouse_move({dx}, {dy}) -> {status}", file=sys.stderr)
        return result

    def mouse_click(self, button: str = "left", device: str = ""):
        """Send a button down+up click."""
        down = self.send_events(
            [{"type": "btn", "data": {"down": True, "button": button}}],
            device
        )
        time.sleep(0.05)
        up = self.send_events(
            [{"type": "btn", "data": {"down": False, "button": button}}],
            device
        )
        d_status = "ok" if "return" in down else str(down.get("error", ""))
        u_status = "ok" if "return" in up else str(up.get("error", ""))
        print(f"  mouse_click({button}) -> {d_status}/{u_status}", file=sys.stderr)

    def close(self):
        self.sock.close()


def run_sequence(client: QMPClient, sequence: list, device: str = ""):
    """Run a sequence of (action, *args) tuples."""
    for item in sequence:
        action = item[0]
        if action == "move":
            client.mouse_move_rel(item[1], item[2], device)
        elif action == "click":
            client.mouse_click("left", device)
        elif action == "btn-down":
            client.send_events(
                [{"type": "btn", "data": {"down": True, "button": "left"}}],
                device
            )
        elif action == "btn-up":
            client.send_events(
                [{"type": "btn", "data": {"down": False, "button": "left"}}],
                device
            )
        elif action == "sleep":
            time.sleep(item[1])
        else:
            print(f"  unknown action: {action}", file=sys.stderr)
        time.sleep(0.15)  # inter-event delay


def main():
    parser = argparse.ArgumentParser(
        description="Inject QEMU mouse events via QMP for usb-mouse proof."
    )
    parser.add_argument("--move", nargs=2, type=int, metavar=("DX", "DY"),
                        help="Relative mouse movement")
    parser.add_argument("--click", action="store_true",
                        help="Left button click (down+up)")
    parser.add_argument("--btn-down", action="store_true",
                        help="Left button down")
    parser.add_argument("--btn-up", action="store_true",
                        help="Left button up")
    parser.add_argument("--proof", action="store_true",
                        help="Run standard proof sequence")
    parser.add_argument("--device", default="",
                        help="Input device name (default: broadcast to all)")
    parser.add_argument("--socket", default=QMP_SOCKET,
                        help=f"QMP socket path (default: {QMP_SOCKET})")
    args = parser.parse_args()

    if not (args.move or args.click or args.btn_down or args.btn_up or args.proof):
        args.proof = True

    print(f"Connecting to QEMU QMP at {args.socket}...", file=sys.stderr)
    try:
        client = QMPClient(args.socket)
    except (ConnectionRefusedError, FileNotFoundError) as e:
        print(f"ERROR: cannot connect to QEMU QMP: {e}", file=sys.stderr)
        print("Start QEMU with SEXOS_QEMU_INPUT_INJECT=1 first.", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"ERROR: QMP connection failed: {e}", file=sys.stderr)
        sys.exit(1)

    print("Connected.", file=sys.stderr)

    dev = args.device

    if args.proof:
        # Standard proof sequence: move, click, move again
        print("Running proof sequence:", file=sys.stderr)
        run_sequence(client, [
            ("move", 20, 10),
            ("move", 10, 5),
            ("sleep", 0.3),
            ("click",),
            ("sleep", 0.3),
            ("move", -5, 15),
            ("move", 0, -8),
            ("sleep", 0.2),
            ("click",),
        ], dev)
    else:
        sequence = []
        if args.move:
            sequence.append(("move", args.move[0], args.move[1]))
        if args.btn_down:
            sequence.append(("btn-down",))
        if args.btn_up:
            sequence.append(("btn-up",))
        if args.click:
            sequence.append(("click",))
        if sequence:
            print("Running custom sequence:", file=sys.stderr)
            run_sequence(client, sequence, dev)

    client.close()
    print("Injection complete.", file=sys.stderr)


if __name__ == "__main__":
    main()
