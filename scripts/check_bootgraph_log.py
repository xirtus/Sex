#!/usr/bin/env python3
"""BootGraph serial-log checker (host-side only).

Usage:
  scripts/check_bootgraph_log.py /tmp/sexos.log [--strict-clock] [--allow-fault]
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple


REQUIRED_PD_MARKERS: List[Tuple[str, str]] = [
    ("sexdisplay", "sexdisplay"),
    ("sexdrive", "sexdrive"),
    ("silkshell", "silkshell"),
    ("sexinput", "sexinput"),
    ("sexusb", "sexusb"),
    ("silkbar", "silkbar"),
    ("linen", "linen"),
    ("sexstore", "sexstore"),
    ("quil", "quil"),
    ("sexbell", "sexbell"),
    ("sexfiles", "sexfiles"),
    ("spindle", "spindle"),
]

# If A2 markers landed, require these grants with ok=1 (observed on full boot).
REQUIRED_A2_GRANTS_OK1: List[str] = [
    "from=kernel to=3 slot=SLOT_DISPLAY target=1 ok=1",
    "from=kernel to=3 slot=SLOT_SHELL target=3 ok=1",
    "from=kernel to=4 slot=SLOT_INPUT target=input_ring ok=1",
    "from=kernel to=4 slot=SLOT_SHELL target=3 ok=1",
    "from=kernel to=6 slot=SLOT_DISPLAY target=1 ok=1",
    "from=kernel to=7 slot=SLOT_DISPLAY target=1 ok=1",
    "from=kernel to=3 slot=SLOT_LINEN target=7 ok=1 optional=1",
    "from=kernel to=3 slot=SLOT_QUIL target=9 ok=1 optional=1",
]

CRITICAL_EDGES: List[Tuple[str, str]] = [
    ("silkbar", "sexdisplay"),
    ("silk-shell", "sexdisplay"),
    ("silk-shell", "silkbar"),
    ("silk-shell", "linen"),
    ("sexinput", "silk-shell"),
    ("sexusb", "sexinput"),
    ("linen", "sexdisplay"),
    ("linen", "sexfiles"),
    ("quil", "sexfiles"),
    ("spindle", "silk-shell"),
    ("spindle", "quil"),
]

FAULT_PATTERNS = ["#PF", "#GP", "panic", "fault.kill"]


@dataclass
class GateResult:
    name: str
    passed: bool
    reason: str = ""


@dataclass
class PdRow:
    pd: str
    init_line: Optional[int]
    ready_line: Optional[int]
    state: str


@dataclass
class EdgeRow:
    edge: str
    receiver_ready_line: Optional[int]
    sender_send_line: Optional[int]
    result: str


def find_first_line(lines: List[str], needle: str) -> Optional[int]:
    for i, line in enumerate(lines, 1):
        if needle in line:
            return i
    return None


def find_all_lines(lines: List[str], needle: str) -> List[int]:
    out: List[int] = []
    for i, line in enumerate(lines, 1):
        if needle in line:
            out.append(i)
    return out


def bootgraph_gate(lines: List[str]) -> Tuple[GateResult, List[PdRow], Dict[str, int]]:
    rows: List[PdRow] = []
    ready_lines: Dict[str, int] = {}
    errs: List[str] = []

    for pd, token in REQUIRED_PD_MARKERS:
        init = find_first_line(lines, f"[{token}.init.start]")
        ready = find_first_line(lines, f"[{token}.ready]")
        if ready is not None:
            ready_lines[pd] = ready

        if init is None or ready is None:
            state = "MISSING"
            errs.append(f"{pd} missing {'init' if init is None else 'ready'}")
        elif init >= ready:
            state = "ORDER_FAIL"
            errs.append(f"{pd} init>=ready ({init}>={ready})")
        else:
            state = "OK"

        rows.append(PdRow(pd=pd, init_line=init, ready_line=ready, state=state))

    if errs:
        return GateResult("BOOTGRAPH_GATE", False, "; ".join(errs)), rows, ready_lines
    return GateResult("BOOTGRAPH_GATE", True), rows, ready_lines


def cap_grant_gate(lines: List[str]) -> GateResult:
    begin = find_first_line(lines, "[bootgraph.phase25.begin]")
    complete = find_first_line(lines, "[bootgraph.phase25.complete]")
    if begin is None:
        return GateResult("CAP_GRANT_GATE", False, "missing phase25.begin")
    if complete is None:
        return GateResult("CAP_GRANT_GATE", False, "missing phase25.complete")
    if begin >= complete:
        return GateResult("CAP_GRANT_GATE", False, f"phase25 order invalid ({begin}>={complete})")

    a2_landed = any("[bootgraph.cap.grant" in line for line in lines)
    if a2_landed:
        for marker in REQUIRED_A2_GRANTS_OK1:
            if find_first_line(lines, f"[bootgraph.cap.grant {marker}]") is None:
                return GateResult("CAP_GRANT_GATE", False, f"missing required grant: {marker}")

    return GateResult("CAP_GRANT_GATE", True)


def parse_first_send_edges(lines: List[str]) -> Dict[Tuple[str, str], int]:
    edge_line: Dict[Tuple[str, str], int] = {}
    rx = re.compile(r"\[bootgraph\.edge\.send\s+from=([^\s]+)\s+to=([^\s\]]+).+first=1\]")
    for i, line in enumerate(lines, 1):
        m = rx.search(line)
        if not m:
            continue
        key = (m.group(1), m.group(2))
        if key not in edge_line:
            edge_line[key] = i
    return edge_line


def order_gate(lines: List[str], ready_lines: Dict[str, int]) -> Tuple[GateResult, List[EdgeRow]]:
    phase_complete = find_first_line(lines, "[bootgraph.phase25.complete]")
    edges = parse_first_send_edges(lines)
    rows: List[EdgeRow] = []
    errs: List[str] = []

    for sender, receiver in CRITICAL_EDGES:
        send_line = edges.get((sender, receiver))
        recv_ready = ready_lines.get(receiver)
        edge_name = f"{sender}->{receiver}"

        if send_line is None or recv_ready is None:
            rows.append(EdgeRow(edge=edge_name, receiver_ready_line=recv_ready, sender_send_line=send_line, result="SKIP"))
            continue

        ok = True
        why: List[str] = []
        if recv_ready >= send_line:
            ok = False
            why.append(f"receiver_ready({recv_ready})>=send({send_line})")
        if phase_complete is not None and phase_complete >= send_line:
            ok = False
            why.append(f"phase25.complete({phase_complete})>=send({send_line})")

        if ok:
            rows.append(EdgeRow(edge=edge_name, receiver_ready_line=recv_ready, sender_send_line=send_line, result="OK"))
        else:
            rows.append(EdgeRow(edge=edge_name, receiver_ready_line=recv_ready, sender_send_line=send_line, result="FAIL"))
            errs.append(f"{edge_name} {'; '.join(why)}")

    if errs:
        return GateResult("ORDER_GATE", False, "; ".join(errs)), rows
    return GateResult("ORDER_GATE", True), rows


def clock_gate(lines: List[str], strict_clock: bool) -> Tuple[GateResult, List[str]]:
    warnings: List[str] = []

    # Prefer tick-based markers if present.
    sb_ticks = set()
    for line in lines:
        m = re.search(r"\[silkbar\.clock\.send[^\]]*tick=(\d+)", line)
        if m:
            sb_ticks.add(int(m.group(1)))

    has_recv_tick = any(re.search(r"\[sexdisplay\.clock\.recv[^\]]*tick=\d+", ln) for ln in lines)
    has_apply_ok_tick = any(re.search(r"\[sexdisplay\.clock\.apply[^\]]*tick=\d+[^\]]*ok=1", ln) for ln in lines)
    has_render_tick = any(re.search(r"\[sexdisplay\.clock\.render[^\]]*tick=\d+", ln) for ln in lines)

    tick_markers_present = bool(sb_ticks or has_recv_tick or has_apply_ok_tick or has_render_tick)

    if not tick_markers_present:
        warnings.append("tick-based clock markers not implemented")
        return (GateResult("CLOCK_GATE", not strict_clock, "WARN: " + "; ".join(warnings) if warnings else ""), warnings)

    errs: List[str] = []
    if len(sb_ticks) < 2:
        errs.append("need >=2 unique silkbar.clock.send tick")
    if not has_recv_tick:
        errs.append("missing sexdisplay.clock.recv tick")
    if not has_apply_ok_tick:
        errs.append("missing sexdisplay.clock.apply tick ok=1")
    if not has_render_tick:
        errs.append("missing sexdisplay.clock.render tick")

    if errs:
        if strict_clock:
            return GateResult("CLOCK_GATE", False, "; ".join(errs)), warnings
        warnings.extend(errs)
        return GateResult("CLOCK_GATE", True, "WARN: " + "; ".join(errs)), warnings

    return GateResult("CLOCK_GATE", True), warnings


def fault_gate(lines: List[str], allow_fault: bool) -> GateResult:
    hits: List[str] = []
    for pat in FAULT_PATTERNS:
        if any(pat in line for line in lines):
            hits.append(pat)
    if hits and not allow_fault:
        return GateResult("FAULT_GATE", False, "fault markers found: " + ", ".join(hits))
    if hits and allow_fault:
        return GateResult("FAULT_GATE", True, "WARN: faults allowed by --allow-fault")
    return GateResult("FAULT_GATE", True)


def print_pd_table(rows: List[PdRow]) -> None:
    print("PD,init_line,ready_line,state")
    for r in rows:
        il = "-" if r.init_line is None else str(r.init_line)
        rl = "-" if r.ready_line is None else str(r.ready_line)
        print(f"{r.pd},{il},{rl},{r.state}")


def print_edge_table(rows: List[EdgeRow]) -> None:
    print("EDGE,receiver_ready_line,sender_send_line,result")
    for r in rows:
        rr = "-" if r.receiver_ready_line is None else str(r.receiver_ready_line)
        ss = "-" if r.sender_send_line is None else str(r.sender_send_line)
        print(f"{r.edge},{rr},{ss},{r.result}")


def main() -> int:
    ap = argparse.ArgumentParser(description="BootGraph serial-log checker")
    ap.add_argument("log_path", help="Serial log path, e.g. /tmp/sexos.log")
    ap.add_argument("--strict-clock", action="store_true", help="Fail CLOCK_GATE on missing/incomplete tick markers")
    ap.add_argument("--allow-fault", action="store_true", help="Allow FAULT_GATE patterns without failing")
    args = ap.parse_args()

    log = Path(args.log_path)
    if not log.exists():
        print(f"BOOTGRAPH FAIL: INPUT_GATE log_not_found path={log}")
        return 1

    lines = log.read_text(errors="replace").splitlines()

    bg_gate, pd_rows, ready_map = bootgraph_gate(lines)
    cap_gate = cap_grant_gate(lines)
    ord_gate, edge_rows = order_gate(lines, ready_map)
    clk_gate, _ = clock_gate(lines, args.strict_clock)
    flt_gate = fault_gate(lines, args.allow_fault)

    gates = [bg_gate, cap_gate, ord_gate, clk_gate, flt_gate]
    failed = [g for g in gates if not g.passed]

    if failed:
        first = failed[0]
        print(f"BOOTGRAPH FAIL: {first.name} {first.reason}".strip())
    else:
        print("BOOTGRAPH PASS")

    for g in gates:
        status = "PASS" if g.passed else "FAIL"
        if g.reason:
            print(f"{g.name}: {status} {g.reason}")
        else:
            print(f"{g.name}: {status}")

    print_pd_table(pd_rows)
    print_edge_table(edge_rows)

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
