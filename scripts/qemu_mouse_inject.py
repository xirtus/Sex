#!/usr/bin/env python3
"""
Inject deterministic mouse events into QEMU via QMP for usb-mouse proof.

Connects to /tmp/sexos-qmp.sock, sends a sequence of mouse movements
and button clicks via HMP (human-monitor-command over QMP).

Usage:
    # Start QEMU with injection enabled:
    SEXOS_QEMU_INPUT_INJECT=1 ./dev.sh run

    # In another terminal after desktop appears:
    python3 scripts/qemu_mouse_inject.py

    # Or custom sequence:
    python3 scripts/qemu_mouse_inject.py --move 20 10 --click --move -10 5

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
        # Consume QMP greeting
        self._read_response()
        # Capabilities handshake
        self._cmd({"execute": "qmp_capabilities"})
        resp = self._read_response()
        if "error" in resp:
            print(f"QMP capabilities error: {resp}", file=sys.stderr)
            sys.exit(1)

    def _read_response(self) -> dict:
        """Read one JSON object from QMP socket (newline-delimited)."""
        buf = b""
        while True:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("QMP socket closed")
            buf += chunk
            try:
                obj, _ = json.JSONDecoder().raw_decode(buf.decode())
                return obj
            except json.JSONDecodeError:
                # Wait for more data
                continue

    def _cmd(self, cmd: dict) -> dict:
        """Send a QMP command and return the response."""
        payload = json.dumps(cmd).encode() + b"\n"
        self.sock.sendall(payload)
        return self._read_response()

    def hmp(self, cmdline: str) -> dict:
        """Execute an HMP command via QMP human-monitor-command."""
        return self._cmd({
            "execute": "human-monitor-command",
            "arguments": {"command-line": cmdline}
        })

    def mouse_move_rel(self, dx: int, dy: int):
        """Relative mouse movement via HMP."""
        result = self.hmp(f"mouse_move {dx} {dy}")
        print(f"  mouse_move({dx}, {dy}) -> {result.get('return', 'ok')}", file=sys.stderr)

    def mouse_button(self, state: int):
        """Set mouse button state (1=left, 2=right, 4=middle)."""
        result = self.hmp(f"mouse_button {state}")
        print(f"  mouse_button({state}) -> {result.get('return', 'ok')}", file=sys.stderr)

    def close(self):
        self.sock.close()


def run_sequence(client: QMPClient, sequence: list):
    """Run a sequence of (action, *args) tuples."""
    for item in sequence:
        action = item[0]
        if action == "move":
            client.mouse_move_rel(item[1], item[2])
        elif action == "click":
            client.mouse_button(1)   # down
            time.sleep(0.05)
            client.mouse_button(0)   # up
        elif action == "btn-down":
            client.mouse_button(1)
        elif action == "btn-up":
            client.mouse_button(0)
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
        ])
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
            run_sequence(client, sequence)

    client.close()
    print("Injection complete.", file=sys.stderr)


if __name__ == "__main__":
    main()
