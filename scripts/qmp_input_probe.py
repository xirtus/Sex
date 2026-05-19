#!/usr/bin/env python3
"""
Deterministic QMP input injector for SexOS dev proofs.
Connects to QEMU QMP socket, injects keyboard events via input-send-event.

Usage:
  ./scripts/qmp_input_probe.py [/path/to/qmp.sock] [key1 key2 ...]
  SEXOS_QMP_SOCK=/path/to/qmp.sock ./scripts/qmp_input_probe.py [key1 key2 ...]

Default keys: w a s d Up Down Left Right
Each key is pressed and released with a short delay.

QEMU 11.0.0 KeyValue union format:
  { "type": "qcode", "data": "w" }
"""
import sys
import json
import time
import socket
import os

DEFAULT_QMP_SOCK = "/tmp/sexos-qmp.sock"
argv = sys.argv[1:]
if argv and argv[0].startswith("/"):
    qmp_sock = argv[0]
    keys = argv[1:]
else:
    qmp_sock = os.environ.get("SEXOS_QMP_SOCK", DEFAULT_QMP_SOCK)
    keys = argv
QMP_SOCK = qmp_sock
KEYS = keys if keys else ["w", "a", "s", "d", "up", "down", "left", "right"]
DELAY = 0.05  # seconds between events


def qmp_send(sock, cmd):
    """Send QMP command and return parsed response."""
    payload = json.dumps(cmd).encode() + b"\n"
    sock.sendall(payload)
    # Read until we have a complete JSON object
    buf = b""
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        buf += chunk
        try:
            return json.loads(buf.decode())
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
    return None


def main():
    if not os.path.exists(QMP_SOCK):
        print(f"ERROR: QMP socket not found: {QMP_SOCK}")
        sys.exit(1)

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(10)
    try:
        sock.connect(QMP_SOCK)
    except ConnectionRefusedError:
        print(f"ERROR: Connection refused: {QMP_SOCK}")
        sys.exit(1)

    # Consume QMP greeting (sent on connect before any command)
    greeting = b""
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        greeting += chunk
        try:
            json.loads(greeting.decode())
            break
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue

    # Send qmp_capabilities
    resp = qmp_send(sock, {"execute": "qmp_capabilities"})
    if resp and "return" in resp:
        print("connected")
    else:
        print(f"ERROR: handshake failed: {resp}")
        sock.close()
        sys.exit(1)

    # Inject each key: press + release
    count = 0
    for key in KEYS:
        # Press
        cmd = {
            "execute": "input-send-event",
            "arguments": {
                "events": [
                    {
                        "type": "key",
                        "data": {
                            "down": True,
                            "key": {"type": "qcode", "data": key},
                        },
                    }
                ]
            },
        }
        resp = qmp_send(sock, cmd)
        if resp and "error" in resp:
            print(f"  press {key}: ERROR {resp['error']}")
        else:
            count += 1
        time.sleep(DELAY)

        # Release
        cmd = {
            "execute": "input-send-event",
            "arguments": {
                "events": [
                    {
                        "type": "key",
                        "data": {
                            "down": False,
                            "key": {"type": "qcode", "data": key},
                        },
                    }
                ]
            },
        }
        resp = qmp_send(sock, cmd)
        if resp and "error" in resp:
            print(f"  release {key}: ERROR {resp['error']}")
        else:
            count += 1
        time.sleep(DELAY)

    print(f"injected key sequence: {count} events ({len(KEYS)} keys)")
    sock.close()


if __name__ == "__main__":
    main()
