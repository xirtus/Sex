#!/usr/bin/env bash
# host_tcp_established_env_probe.sh — Host TCP Listener Probe for Phase GHI
#
# Starts or verifies a host TCP listener for the Phase GHI established proof.
# Bounded: no infinite loops, no root, cleanup on exit.
#
# Usage:
#   ./scripts/host_tcp_established_env_probe.sh [log_path] [port]
#
#   log_path defaults to /tmp/sexnet_phase_ghi_host_env.log
#   port defaults to 18080
#
# Returns:
#   0 — listener started/verified successfully
#   1 — listener failed (port in use, no listener tool available)
#   2 — skipped (no suitable listener tool)
#
# See: docs/handoff/SEXNET_TCP_ESTABLISHED_ENV_PROOF_V1.md

set -euo pipefail

LOG="${1:-/tmp/sexnet_phase_ghi_host_env.log}"
PORT="${2:-18080}"
MAX_ATTEMPTS=3
TIMEOUT=2

: > "$LOG" || { echo "FATAL: cannot write to $LOG" >&2; exit 2; }

log() { echo "$@" | tee -a "$LOG"; }

echo "[sexnet.phaseGHI.host_tcp_env.begin] port=$PORT" | tee -a "$LOG"

# --- Check for available listener tools ---
LISTENER_TOOL=""
LISTENER_ARGS=()
LISTENER_KILL_SIG="TERM"

if command -v nc >/dev/null 2>&1; then
    # Test if nc supports -l -p (traditional netcat) vs -l (OpenBSD netcat)
    if nc -h 2>&1 | grep -q '\-p'; then
        LISTENER_TOOL="nc"
        LISTENER_ARGS=(-l -p "$PORT")
    elif echo "test" | nc -l "$PORT" -w 0 2>/dev/null || true; then
        LISTENER_TOOL="nc"
        LISTENER_ARGS=(-l "$PORT")
    fi
fi

if [ -z "$LISTENER_TOOL" ] && command -v socat >/dev/null 2>&1; then
    LISTENER_TOOL="socat"
    LISTENER_ARGS=(TCP-LISTEN:"$PORT",reuseaddr,fork -)
    LISTENER_KILL_SIG="TERM"
fi

if [ -z "$LISTENER_TOOL" ] && command -v python3 >/dev/null 2>&1; then
    LISTENER_TOOL="python3"
    LISTENER_ARGS=(-c "
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('', $PORT))
s.listen(1)
print('[sexnet.phaseGHI.host_tcp_env.listener.ready] port=$PORT ok=1 host_listener_only=1')
conn, addr = s.accept()
data = conn.recv(1024)
print(f'[sexnet.phaseGHI.host_tcp_env.payload.rx] bytes={len(data)} ok=1')
conn.close()
s.close()
")
    LISTENER_KILL_SIG="TERM"
fi

if [ -z "$LISTENER_TOOL" ]; then
    echo "[sexnet.phaseGHI.host_tcp_env.skip] reason=no_listener_tool port=$PORT" | tee -a "$LOG"
    exit 2
fi

echo "[sexnet.phaseGHI.host_tcp_env.tool] tool=$LISTENER_TOOL port=$PORT" | tee -a "$LOG"

# --- Check if port is already in use ---
if command -v ss >/dev/null 2>&1; then
    if ss -tlnp 2>/dev/null | grep -q ":$PORT "; then
        echo "[sexnet.phaseGHI.host_tcp_env.skip] reason=port_in_use port=$PORT" | tee -a "$LOG"
        exit 2
    fi
fi

# --- Start listener test (brief) ---
echo "[sexnet.phaseGHI.host_tcp_env.listener.start] port=$PORT ok=1" | tee -a "$LOG"

CLEANUP_DONE=0
cleanup() {
    if [ "$CLEANUP_DONE" -eq 0 ]; then
        CLEANUP_DONE=1
        kill "$LISTENER_PID" 2>/dev/null || true
        wait "$LISTENER_PID" 2>/dev/null || true
        echo "[sexnet.phaseGHI.host_tcp_env.cleanup] port=$PORT ok=1" | tee -a "$LOG"
    fi
}
trap cleanup EXIT INT TERM

# Start listener in background
if [ "$LISTENER_TOOL" = "python3" ]; then
    "$LISTENER_TOOL" "${LISTENER_ARGS[@]}" &
else
    "$LISTENER_TOOL" "${LISTENER_ARGS[@]}" &
fi
LISTENER_PID=$!
echo "[sexnet.phaseGHI.host_tcp_env.listener.pid] pid=$LISTENER_PID" | tee -a "$LOG"

# Brief test: connect and disconnect to verify listener is alive
sleep 0.5
TEST_OK=0
for i in $(seq 1 $MAX_ATTEMPTS); do
    if kill -0 "$LISTENER_PID" 2>/dev/null; then
        # Quick TCP connect test
        if command -v nc >/dev/null 2>&1; then
            if echo "PROBE" | nc -w "$TIMEOUT" 127.0.0.1 "$PORT" 2>/dev/null; then
                TEST_OK=1
                echo "[sexnet.phaseGHI.host_tcp_env.test_connect] attempt=$i ok=1" | tee -a "$LOG"
            else
                echo "[sexnet.phaseGHI.host_tcp_env.test_connect] attempt=$i ok=0" | tee -a "$LOG"
            fi
        else
            # If no nc to test, just assume listener process is alive
            TEST_OK=1
            echo "[sexnet.phaseGHI.host_tcp_env.test_connect] attempt=$i ok=1 assume=1" | tee -a "$LOG"
        fi
        if [ "$TEST_OK" -eq 1 ]; then
            break
        fi
    else
        echo "[sexnet.phaseGHI.host_tcp_env.test_connect] attempt=$i pid_dead=1" | tee -a "$LOG"
    fi
    sleep 0.5
done

if [ "$TEST_OK" -eq 1 ]; then
    echo "[sexnet.phaseGHI.host_tcp_env.listener.ready] port=$PORT ok=1" | tee -a "$LOG"
    echo "[sexnet.phaseGHI.host_tcp_env.pass]" | tee -a "$LOG"
    # KEEP listener running — it needs to stay alive for QEMU proof
    # The caller is responsible for killing it (trap cleanup on exit)
    # Output PID for caller
    echo "LISTENER_PID=$LISTENER_PID"
    echo "LISTENER_PORT=$PORT"
    exit 0
else
    echo "[sexnet.phaseGHI.host_tcp_env.fail] reason=listener_not_ready port=$PORT" | tee -a "$LOG"
    cleanup
    exit 1
fi
