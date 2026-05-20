#!/usr/bin/env python3
"""
Deterministic QMP input injector for SexOS dev proofs.

Usage:
  ./scripts/qmp_input_probe.py /path/to/qmp.sock mouse
  ./scripts/qmp_input_probe.py /path/to/qmp.sock key w a s d
  SEXOS_QMP_SOCK=/path/to/qmp.sock ./scripts/qmp_input_probe.py mouse
"""

import json
import os
import socket
import sys
import time
from typing import Any, Dict, List, Optional, Tuple

DEFAULT_QMP_SOCK = "/tmp/sexos-qmp.sock"
DEFAULT_TIMEOUT = float(os.environ.get("SEXOS_QMP_TIMEOUT", "5.0"))
DEFAULT_EVENT_DELAY = float(os.environ.get("SEXOS_QMP_EVENT_DELAY", "0.05"))


class QMPError(Exception):
    pass


def parse_args(argv: List[str]) -> Tuple[str, str, List[str], float]:
    qmp_sock = os.environ.get("SEXOS_QMP_SOCK", DEFAULT_QMP_SOCK)
    delay = DEFAULT_EVENT_DELAY
    rest = list(argv)

    if rest and rest[0].startswith("/"):
        qmp_sock = rest.pop(0)

    i = 0
    while i < len(rest):
        tok = rest[i]
        if tok == "--delay":
            if i + 1 >= len(rest):
                raise QMPError("missing value for --delay")
            try:
                delay = float(rest[i + 1])
            except ValueError as exc:
                raise QMPError(f"invalid --delay value: {rest[i + 1]!r}") from exc
            if delay < 0:
                raise QMPError("--delay must be >= 0")
            del rest[i : i + 2]
            continue
        i += 1

    mode = "mouse"
    tokens: List[str] = []
    if rest:
        first = rest[0].lower()
        if first in {"mouse", "key", "keyboard"}:
            mode = "key" if first in {"key", "keyboard"} else "mouse"
            tokens = rest[1:]
        else:
            # Backward compatibility: plain key list means keyboard mode.
            mode = "key"
            tokens = rest

    return qmp_sock, mode, tokens, delay


def summarize(msg: Dict[str, Any]) -> str:
    if "error" in msg:
        err = msg["error"]
        if isinstance(err, dict):
            return f"error class={err.get('class')} desc={err.get('desc')}"
        return f"error {err}"
    if "return" in msg:
        ret = msg["return"]
        if isinstance(ret, dict):
            return f"return keys={','.join(sorted(ret.keys())) or '<none>'}"
        if isinstance(ret, list):
            return f"return list[{len(ret)}]"
        return f"return {ret!r}"
    if "event" in msg:
        return f"event {msg.get('event')}"
    return f"keys={','.join(sorted(msg.keys()))}"


def recv_line(sock: socket.socket, buf: bytearray, timeout: float) -> Dict[str, Any]:
    deadline = time.monotonic() + timeout
    while True:
        nl = buf.find(b"\n")
        if nl != -1:
            line = bytes(buf[:nl]).strip()
            del buf[: nl + 1]
            if not line:
                continue
            try:
                return json.loads(line.decode("utf-8", errors="strict"))
            except json.JSONDecodeError as exc:
                raise QMPError(f"invalid QMP JSON line: {line!r}: {exc}") from exc

        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("timeout waiting for QMP data")

        sock.settimeout(remaining)
        chunk = sock.recv(4096)
        if not chunk:
            raise QMPError("QMP socket closed by peer")
        buf.extend(chunk)


def wait_for_response(sock: socket.socket, buf: bytearray, timeout: float) -> Dict[str, Any]:
    while True:
        msg = recv_line(sock, buf, timeout)
        if "event" in msg:
            print(f"[qmp.recv] {summarize(msg)}")
            continue
        return msg


def send_cmd(sock: socket.socket, buf: bytearray, cmd: Dict[str, Any], timeout: float) -> Optional[Dict[str, Any]]:
    payload = json.dumps(cmd, separators=(",", ":"))
    print(f"[qmp.send] {payload}")
    sock.sendall(payload.encode("utf-8") + b"\n")
    try:
        resp = wait_for_response(sock, buf, timeout)
    except TimeoutError:
        print("[qmp.recv] timeout waiting for command response")
        return None
    print(f"[qmp.recv] {summarize(resp)}")
    return resp


def key_events(keys: List[str]) -> List[Tuple[str, Dict[str, Any]]]:
    use_keys = keys if keys else ["w", "a", "s", "d", "up", "down", "left", "right"]
    out: List[Tuple[str, Dict[str, Any]]] = []
    for key in use_keys:
        out.append(
            (
                f"key press {key}",
                {
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
                },
            )
        )
        out.append(
            (
                f"key release {key}",
                {
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
                },
            )
        )
    return out


def mouse_events() -> List[Tuple[str, Dict[str, Any]]]:
    return [
        (
            "mouse move +24,+12",
            {
                "execute": "input-send-event",
                "arguments": {
                    "events": [
                        {
                            "type": "rel",
                            "data": {"axis": "x", "value": 24},
                        },
                        {
                            "type": "rel",
                            "data": {"axis": "y", "value": 12},
                        },
                    ]
                },
            },
        ),
        (
            "mouse left down",
            {
                "execute": "input-send-event",
                "arguments": {
                    "events": [
                        {
                            "type": "btn",
                            "data": {"down": True, "button": "left"},
                        }
                    ]
                },
            },
        ),
        (
            "mouse left up",
            {
                "execute": "input-send-event",
                "arguments": {
                    "events": [
                        {
                            "type": "btn",
                            "data": {"down": False, "button": "left"},
                        }
                    ]
                },
            },
        ),
    ]


def main() -> int:
    qmp_sock, mode, tokens, delay = parse_args(sys.argv[1:])

    if not os.path.exists(qmp_sock):
        print(f"ERROR: QMP socket not found: {qmp_sock}")
        return 1

    print(
        f"[qmp] socket={qmp_sock} mode={mode} timeout={DEFAULT_TIMEOUT:.2f}s "
        f"delay={delay:.2f}s"
    )

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    buf = bytearray()
    try:
        sock.settimeout(DEFAULT_TIMEOUT)
        sock.connect(qmp_sock)
        print("[qmp] connected")

        try:
            greeting = wait_for_response(sock, buf, DEFAULT_TIMEOUT)
        except TimeoutError:
            print(
                "ERROR: timeout waiting for QMP greeting after connect; "
                "socket exists but server did not send greeting"
            )
            return 2

        if "QMP" not in greeting:
            print(f"ERROR: unexpected greeting: {greeting}")
            return 2
        print(f"[qmp.greeting] {summarize(greeting)}")

        caps = send_cmd(sock, buf, {"execute": "qmp_capabilities"}, DEFAULT_TIMEOUT)
        if not caps or "error" in caps:
            print(f"ERROR: qmp_capabilities failed: {caps}")
            return 3

        commands = mouse_events() if mode == "mouse" else key_events(tokens)
        ok = 0
        for label, cmd in commands:
            print(f"[qmp.event] {label}")
            resp = send_cmd(sock, buf, cmd, DEFAULT_TIMEOUT)
            if not resp:
                print(f"ERROR: no response for event: {label}")
                continue
            if "error" in resp:
                print(f"ERROR: event rejected: {label}")
                continue
            ok += 1
            time.sleep(delay)

        print(f"[qmp.summary] attempted={len(commands)} succeeded={ok} mode={mode}")
        return 0 if ok > 0 else 4
    except (ConnectionRefusedError, FileNotFoundError) as exc:
        print(f"ERROR: connect failed: {exc}")
        return 1
    except TimeoutError as exc:
        print(f"ERROR: timeout: {exc}")
        return 2
    except QMPError as exc:
        print(f"ERROR: {exc}")
        return 2
    finally:
        sock.close()


if __name__ == "__main__":
    sys.exit(main())
