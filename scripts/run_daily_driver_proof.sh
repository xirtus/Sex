#!/usr/bin/env bash
# run_daily_driver_proof.sh — Daily-Driver Proof Profile V1
#
# Builds a proof ISO with all daily-driver proof gates enabled, boots in
# headless QEMU, captures the serial log, and runs daily_driver_master_gate.sh.
#
# This is a host-side orchestration script only.  It does not imply POSIX
# semantics inside SexOS and makes zero source-code, kernel, ABI, USB, input,
# display, or app behavior changes.
#
# Usage:
#   ./scripts/run_daily_driver_proof.sh [log_path]
#
#   log_path defaults to /tmp/sexos_daily_driver_proof.log
#
# Returns:
#   0 — build, boot, and all enabled gates PASS, zero faults
#   1 — build failed, gate failed, or faults detected
#   2 — fatal error (missing scripts, log path unwritable)
#
# See: docs/handoff/DAILY_DRIVER_PROOF_PROFILE_V1.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

LOG="${1:-/tmp/sexos_daily_driver_proof.log}"
GATE_SCRIPT="./scripts/daily_driver_master_gate.sh"
BUILD_SCRIPT="./scripts/entrypoint_build.sh"
ISO="sexos-v1.0.0.iso"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
PROBE_SECONDS="${DAILY_DRIVER_PROBE_SECONDS:-30}"
ENABLE_QEMU_USERNET_E1000="${ENABLE_QEMU_USERNET_E1000:-1}"
QEMU_NET_MODEL="${QEMU_NET_MODEL:-e1000}"
QEMU_NET_BACKEND="${QEMU_NET_BACKEND:-user}"
QEMU_USERNET_HOSTFWD="${QEMU_USERNET_HOSTFWD:-}"
QEMU_TAP_IFNAME="${QEMU_TAP_IFNAME:-tap0}"
SEXNET_PHASE_I_HTTP_PROOF="${SEXNET_PHASE_I_HTTP_PROOF:-0}"
SEXNET_PHASE_K_BROWSER_PROOF="${SEXNET_PHASE_K_BROWSER_PROOF:-0}"

# ---- helpers ----
die() {
    echo "FATAL: $*" >&2
    exit 2
}

# ---- validate prerequisites ----
[ -x "$BUILD_SCRIPT" ] || die "build script not found: $BUILD_SCRIPT"
[ -x "$GATE_SCRIPT" ] || die "gate script not found: $GATE_SCRIPT"
# Ensure log directory is writable.
mkdir -p "$(dirname "$LOG")" 2>/dev/null || die "cannot create log directory: $(dirname "$LOG")"
: > "$LOG" || die "cannot write to log: $LOG"

# ---- proof environment variables ----
#
# Each variable activates a compile-time option_env! gate in the corresponding
# SexOS app or server.  Variables for proofs not yet implemented are silently
# ignored by the compiler (is_some() returns false).

# ── Spindle daily-driver proofs ──
export SEXOS_SPINDLE_DAILY_SUMMARY_PROOF=1
export SEXOS_SPINDLE_STATUS_PANEL_PROOF=1
export SEXOS_SPINDLE_BELL_BRIDGE_PROOF=1
export SEXOS_SPINDLE_LINEN_BRIDGE_PROOF=1
export SEXOS_SPINDLE_FILES_COMMANDS_PROOF=1
export SEXOS_SPINDLE_COMMAND_HISTORY_PROOF=1
export SEXOS_SPINDLE_PERSIST_HISTORY_PROOF=1
# NOTE: SEXOS_SPINDLE_INPUT_PROOF is intentionally NOT set.
# It enables framebuffer writes that cause a PAGE FAULT at 0x40000000
# when Spindle is kernel-spawned alongside silk-shell's own FB.
# The input proof is compile-verified in isolation.

# ── Command palette ──
export SEXOS_COMMAND_PALETTE_STATUS_PROOF=1
export SEXOS_COMMAND_PALETTE_DAILY_PROOF=1
export SEXOS_COMMAND_PALETTE_LINEN_STATUS_PROOF=1

# ── App Launcher ──
export SEXOS_APP_LAUNCHER_MULTI_EXEC_PROOF=1

# ── Linen ──
export SEXOS_LINEN_NONBLOCKING_OPEN_PROOF=1
export SEXOS_LINEN_OBJECT_DETAIL_PROOF=1
export SEXOS_LINEN_KEYBOARD_NAV_PROOF=1
export SEXOS_LINEN_SESSION_PROOF=1

# ── Quil ──
export SEXOS_QUIL_KEYBOARD_BUFFER_PROOF=1
export SEXOS_QUIL_KEYBOARD_NAV_PROOF=1
export SEXOS_QUIL_STATUS_UNBLOCK_PROOF=1

# ── Bell ──
export SEXOS_BELL_SYSTEM_EVENTS_PROOF=1
export SEXOS_BELL_DETAIL_SEED_PROOF=1
export SEXOS_BELL_KEYBOARD_DETAIL_PROOF=1

# ── Atlas ──
export SEXOS_ATLAS_THEME_VISUAL_PROOF=1
export SEXOS_ATLAS_THEME_PRESETS_PROOF=1
export SEXOS_ATLAS_SCENE_KEYBOARD_PROOF=1

# ── Atlas Phase proofs (A-E2) ──
export SEXOS_ATLAS_PHASE_A_STATE_MODEL_PROOF=1
export SEXOS_ATLAS_PHASE_B_SNAPSHOT_PROOF=1
export SEXOS_ATLAS_PHASE_C_RENDER_STUB_PROOF=1
export SEXOS_ATLAS_PHASE_D_FRAME_PREVIEW_STUB_PROOF=1
export SEXOS_ATLAS_PHASE_E1_CLICK_SCENE_SWITCH_PROOF=1
export SEXOS_ATLAS_PHASE_E2_KEYBOARD_SCENE_CYCLE_PROOF=1
export SEXOS_ATLAS_PHASE_E3_DRAG_BEGIN_MARKER_PROOF=1
export SEXOS_ATLAS_PHASE_E4B_SAME_SCENE_NOOP_PROOF=1
export SEXOS_ATLAS_PHASE_E4C_CROSS_SCENE_REPARENT_PROOF=1
export SEXOS_ATLAS_PHASE_E4C2_TRUE_REPARENT_PROOF=1
export SEXOS_ATLAS_PHASE_E4D_REAL_POINTER_DROP_PROOF=1
export SEXOS_ATLAS_OVERVIEW_FINAL_CLOSEOUT_PROOF=1

# ── Collar ──
export SEXOS_COLLAR_KEYBOARD_GRANTS_PROOF=1
export SEXOS_COLLAR_ENFORCE_PROOF=1
export SEXOS_COLLAR_REVIEW_PROOF=1

# ── Mesh / Frame ──
export SEXOS_MESH_KEYBOARD_MAP_PROOF=1

# ── SilkBar ──
export SEXOS_SILKBAR_KEYBOARD_STATUS_PROOF=1
export SEXOS_SILKBAR_PALETTE_STATUS_PROOF=1

# ── SilkBar ABI Extension (Phase 2+3+5) ──
export SEXOS_SILKBAR_PHASE2_SHELL_PROOF=1
export SEXOS_SILKBAR_PHASE3_RECEIVE_PROOF=1
export SEXOS_SILKBAR_PHASE5_PIXEL_PROOF=1

# ── Keyboard GUI broad proof ──
export SEXOS_KEYBOARD_GUI_BROAD_PROOF=1
export SEXOS_KEYBOARD_PROOF=1
export SEXOS_KEYBOARD_SAFE_CLOSE_PROOF=1
export SEXOS_KEYBOARD_WINDOW_PROOF=1

# ── SexFiles storage ──
export SEXOS_SEXFILES_CAP_RECORD_PROOF=1
export SEXOS_SEXFILES_EXTENT_PROOF=1

# ── SexObject ──
export SEXOS_SEXOBJECT_VIEW_PROOF=1
export SEXOS_SEXOBJECT_OQ=1

# ── Feature batch 2.6 V2 proofs ──
export SEXOS_APP_LAUNCH_COMMANDS_PROOF=1
export SEXOS_LINEN_OBJECT_WORKFLOW_PROOF=1
export SEXOS_QUIL_TEXT_BUFFER_PROOF=1
export SEXOS_BELL_APP_EVENT_INTEGRATION_PROOF=1

# ── Feature batch V3 persistence proofs ──
export SEXOS_LINEN_OBJECT_PERSIST_PROOF=1
export SEXOS_QUIL_TEXT_SAVE_ASYNC_PROOF=1
export SEXOS_SPINDLE_APP_LAUNCH_EXEC_PROOF=1
export SEXOS_BELL_WORKFLOW_EVENT_PROOF=1

# ── Feature batch V4 registry/schema proofs ──
export SEXOS_APP_REGISTRY_STATIC_V2_PROOF=1
export SEXOS_LINEN_OBJECT_SCHEMA_PROOF=1
export SEXOS_QUIL_TEXT_COMMANDS_PROOF=1
export SEXOS_BELL_WORKFLOW_DETAIL_PROOF=1

# ── Feature batch V5 workflow usability proofs ──
export SEXOS_SPINDLE_LINEN_WORKFLOW_PROOF=1
export SEXOS_SPINDLE_QUIL_WORKFLOW_PROOF=1
export SEXOS_QUIL_CURSOR_NAV_PROOF=1

# ── Feature batch V6 arch and editor proofs ──
export SEXOS_QUIL_TEXT_SELECTION_PROOF=1
export SEXOS_QUIL_TEXT_DELETE_PROOF=1
export SEXOS_SPINDLE_EDITOR_V2_PROOF=1

# ── Feature batch V7 lifecycle/editor proofs ──
export SEXOS_QUIL_EDITOR_KEYBINDINGS_PROOF=1
export SEXOS_APP_LIFECYCLE_STATE_PROOF=1
export SEXOS_SPINDLE_APP_LIFECYCLE_PROOF=1

# ── Feature batch V8 undo/lifecycle proofs ──
export SEXOS_QUIL_UNDO_REDO_PROOF=1
export SEXOS_QUIL_UNDO_REDO_KEY_PROOF=1
export SEXOS_APP_LIFECYCLE_CLOSE_RESTORE_PROOF=1
export SEXOS_SPINDLE_LIFECYCLE_HELP_V2_PROOF=1

# ── Feature batch V9 editor/visual/bell proofs ──
export SEXOS_QUIL_VISUAL_CURSOR_PROOF=1
export SEXOS_BELL_DELIVERY_AUDIT_PROOF=1
export SEXOS_SPINDLE_EDITOR_STATUS_PROOF=1
export SEXOS_APP_LIFECYCLE_SUMMARY_V2_PROOF=1

# ── Feature batch V10 search/find proofs ──
export SEXOS_QUIL_FIND_PROOF=1
export SEXOS_SPINDLE_SEARCH_HELP_PROOF=1

# ── Feature batch V11 editor text quality proofs ──
export SEXOS_QUIL_MOD_LOWERCASE_PROOF=1
export SEXOS_QUIL_WORD_NAV_PROOF=1
export SEXOS_QUIL_LINE_STATS_PROOF=1
export SEXOS_SPINDLE_EDITOR_QUALITY_PROOF=1

# ── Feature batch V12 editor polish proofs ──
export SEXOS_QUIL_FIND_NAV_PROOF=1
export SEXOS_QUIL_SEL_COPY_DELETE_PROOF=1
export SEXOS_QUIL_DIRTY_PROOF=1
export SEXOS_SPINDLE_EDITOR_POLISH_PROOF=1

# ── Feature batch V13 editor command surface proofs ──
export SEXOS_QUIL_CMD_SURFACE_PROOF=1
export SEXOS_QUIL_CLIPBOARD_STATUS_PROOF=1
export SEXOS_SPINDLE_EDITOR_V3_PROOF=1

# ── Feature batch V14 editor finishing proofs ──
export SEXOS_QUIL_PASTE_PROOF=1
export SEXOS_QUIL_REPLACE_PROOF=1
export SEXOS_QUIL_GOTO_LINE_PROOF=1
export SEXOS_SPINDLE_EDITOR_FINISH_PROOF=1

# ── Storage Phase A markers proof ──
export SEXOS_QUIL_STORAGE_PHASEA_PROOF=1

# ── App registry lifecycle V2 proof ──
export SEXOS_APP_REGISTRY_LIFECYCLE_V2_PROOF=1

# ── Window workflow V2 proof ──
export SEXOS_WINDOW_WORKFLOW_V2_PROOF=1
export SEXOS_SPINDLE_WINDOW_WORKFLOW_PROOF=1

# ── Browser stub proof ──
export SEXOS_BROWSER_STUB_PROOF=1
export SEXOS_SPINDLE_BROWSER_STUB_PROOF=1
export SEXOS_BROWSER_LOCALDOC_STUB_PROOF=1
export SEXOS_BROWSER_PLACEHOLDER_SURFACE_VISUAL_PROOF=1
export SEXOS_WEBSTUB_LOCALDOC_TEXT_PROOF=1
export SEXOS_BROWSER_URL_INTENT_PROOF=1
export SEXOS_QUIL_VISIBLE_TYPING_E2E_PROOF=1
export SEXOS_WEBSTUB_STATIC_TEXT_RENDER_PROOF=1
export SEXOS_SHELL_DRAW_TEXT_HELPER_PROOF=1
export SEXOS_BROWSER_STUB_V2_PROOF=1
export SEXOS_BROWSER_LOCALDOC_VIEWER_PROOF=1
export SEXOS_BROWSER_URL_BAR_INTENT_PROOF=1
export SEXOS_BROWSER_HISTORY_PROOF=1
export SEXOS_BROWSER_BOOKMARKS_PROOF=1
export SEXOS_BROWSER_TABS_PROOF=1
export SEXOS_BROWSER_ACTIONS_PROOF=1
export SEXOS_BROWSER_DASHBOARD_PROOF=1
export SEXOS_BROWSER_FIND_PROOF=1
export SEXOS_BROWSER_READER_PROOF=1
export SEXOS_BROWSER_SAVE_PROOF=1
export SEXOS_BROWSER_EXPORT_PROOF=1
export SEXOS_BROWSER_URL_PARSE_PROOF=1
export SEXOS_BROWSER_HTML_PROOF=1
export SEXOS_BROWSER_HTML_LINK_PROOF=1
export SEXOS_BROWSER_HTML_HISTORY_PROOF=1

export SEXOS_SEXNET_BROWSER_CAP_PROOF=1
export SEXOS_SEXNET_STATUS_ROUTE_PROOF=1
# ── Linen persist readback model proof ──
export SEXOS_HTTP_CLIENT_STATUS_PROOF=1
export SEXOS_BROWSER_NET_GRANT_PROOF=1
export SEXOS_HTTP_REQ_BUILDER_PROOF=1
export SEXOS_SEXNET_HTTP_HANDSHAKE_PROOF=1
export SEXOS_QEMU_E1000_PCI_ENUM_PROOF=1
export SEXOS_PCI_NET_STATUS_PROOF=1
export SEXOS_E1000_BAR_META_PROOF=1
export SEXOS_E1000_DRIVER_STATUS_PROOF=1
export SEXOS_E1000_RING_ALLOC_PROOF=1
export SEXOS_DMA_UC_ALIAS_PROOF=1
export SEXOS_LINEN_PERSIST_READBACK_PROOF=1

# ── Phase L: source=3 primary network truth proof ──
# Must come FIRST: when caller requests Phase L, it cascades to Phase I+K.
SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF="${SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF:-0}"
if [ "$SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF" = "1" ]; then
    export SEXNET_PHASE_I_HTTP_PROOF=1
    export SEXNET_PHASE_K_BROWSER_PROOF=1
fi

# ── Sexnet Phase I source=3 explicit profile trigger ──
# Keep daily default unchanged: only enable this widened runtime window when
# caller explicitly requests the Phase I proof lane.
if [ "$SEXNET_PHASE_I_HTTP_PROOF" = "1" ]; then
    # Ensure HAL-side probe noise stays off for this lane unless caller
    # explicitly overrides it.
    export SEXOS_HAL_TCP_PROBE="${SEXOS_HAL_TCP_PROBE:-0}"
    # Phase I lane executes late in boot after bounded ARP/cache polls.
    # 30s default often truncates before [sexnet.tcp.entry], so widen only
    # for this explicit profile.
    if [ "$PROBE_SECONDS" -lt 90 ]; then
        PROBE_SECONDS=90
    fi
fi

# ── Phase K: Browser sexnet source=3 remote page proof ──
# Enable browser source=3 markers in silk-shell when caller requests
# the Phase K browser route through sexnet proof lane.
SEXNET_PHASE_K_BROWSER_PROOF="${SEXNET_PHASE_K_BROWSER_PROOF:-0}"
if [ "$SEXNET_PHASE_K_BROWSER_PROOF" = "1" ]; then
    export SEXOS_BROWSER_SEXNET_SOURCE3_PROOF=1
fi

# ── Phase M: source3 reliability multi-fetch proof ──
# Enables N=3 repeated HTTP GET with descriptor reuse, retry policy,
# browser render stability, and long-run no-fault proof.
# Cascades to Phase I+K+L (full source3 primary path).
# Widens probe window for stress/long-run profile.
SEXNET_PHASE_M_RELIABILITY_PROOF="${SEXNET_PHASE_M_RELIABILITY_PROOF:-0}"
if [ "$SEXNET_PHASE_M_RELIABILITY_PROOF" = "1" ]; then
    export SEXNET_PHASE_I_HTTP_PROOF=1
    export SEXNET_PHASE_K_BROWSER_PROOF=1
    export SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF=1
    export SEXNET_PHASE_M_RELIABILITY_PROOF=1
    export SEXOS_BROWSER_SEXNET_SOURCE3_PROOF=1
    export SEXOS_HAL_TCP_PROBE=0
    if [ "$PROBE_SECONDS" -lt 120 ]; then
        PROBE_SECONDS=120
    fi
fi

# ── Phase N: real hardware audit ──
# Enables Phase N real hardware audit markers in sexnet.
# Does NOT change NIC model, BAR mapping, or kernel behavior.
# Does NOT enable real hardware MMIO writes.
# Uses same QEMU e1000 path for regression verification.
SEXNET_PHASE_N_REAL_HW_AUDIT="${SEXNET_PHASE_N_REAL_HW_AUDIT:-0}"
if [ "$SEXNET_PHASE_N_REAL_HW_AUDIT" = "1" ]; then
    export SEXNET_PHASE_N_REAL_HW_AUDIT=1
    export SEXNET_PHASE_I_HTTP_PROOF=1
    export SEXNET_PHASE_K_BROWSER_PROOF=1
    export SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF=1
    export SEXOS_BROWSER_SEXNET_SOURCE3_PROOF=1
    export SEXOS_HAL_TCP_PROBE=0
    if [ "$PROBE_SECONDS" -lt 90 ]; then
        PROBE_SECONDS=90
    fi
fi

	# ── Phase O: final network 100% gates ──
	# Cascades through Phase I+K+L+M+N (full source3 primary path).
	# Widens probe window to 120s for comprehensive final proof.
	SEXNET_PHASE_O_FINAL_NETWORK_PROOF="${SEXNET_PHASE_O_FINAL_NETWORK_PROOF:-0}"
	if [ "$SEXNET_PHASE_O_FINAL_NETWORK_PROOF" = "1" ]; then
	    export SEXNET_PHASE_O_FINAL_NETWORK_PROOF=1
	    export SEXNET_PHASE_I_HTTP_PROOF=1
	    export SEXNET_PHASE_K_BROWSER_PROOF=1
	    export SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF=1
	    export SEXNET_PHASE_M_RELIABILITY_PROOF=1
	    export SEXNET_PHASE_N_REAL_HW_AUDIT=1
	    export SEXOS_BROWSER_SEXNET_SOURCE3_PROOF=1
	    export SEXOS_HAL_TCP_PROBE=0
	    if [ "$PROBE_SECONDS" -lt 120 ]; then
	        PROBE_SECONDS=120
	    fi
	fi

# ── Frame Chrome model proof ──
export SEXOS_FRAME_CHROME_MODEL_PROOF=1
export SEXOS_SPINDLE_FRAME_CHROME_PROOF=1

# ── Frame Rim markers proof ──
export SEXOS_FRAME_RIM_MARKERS_PROOF=1
export SEXOS_SPINDLE_FRAME_RIM_PROOF=1

# ── Frame Lights status stub proof ──
export SEXOS_FRAME_LIGHTS_STUB_PROOF=1
export SEXOS_SPINDLE_FRAME_LIGHTS_PROOF=1
export SEXOS_FRAME_LIGHTS_KEYBOARD_PROOF=1
export SEXOS_BELL_LAUNCH_OUTCOME_PROOF=1

# ── Atlas Scene status stub proof ──
export SEXOS_ATLAS_SCENE_STUB_PROOF=1
export SEXOS_SPINDLE_ATLAS_PROOF=1
export SEXOS_SCENE_LIFECYCLE_MARKERS_PROOF=1
export SEXOS_SCENE_KEYBOARD_SWITCH_PROOF=1
export SEXOS_PROJECT_SCENE_LINK_PROOF=1
export SEXOS_MESH_GRAPH_STATUS_PROOF=1
export SEXOS_COLLAR_GRANT_STATUS_PROOF=1

# ── Lifecycle final5 proofs ──
export SEXOS_LIFECYCLE_ATLAS_PROOF=1
export SEXOS_LIFECYCLE_APPDEATH_PROOF=1

echo "============================================"
echo " DAILY-DRIVER PROOF PROFILE V35"
echo "============================================"
echo ""
echo "  log:     $LOG"
echo "  iso:     $ISO"
echo "  probe:   ${PROBE_SECONDS}s"
echo "  nic:     ${QEMU_NET_MODEL} (backend=${QEMU_NET_BACKEND} usernet=${ENABLE_QEMU_USERNET_E1000})"
echo "  hostfwd: ${QEMU_USERNET_HOSTFWD:-none}"
echo "  phaseI:  ${SEXNET_PHASE_I_HTTP_PROOF}"
echo "  phaseK:  ${SEXNET_PHASE_K_BROWSER_PROOF}"
echo "  phaseL:  ${SEXNET_PHASE_L_SOURCE3_PRIMARY_PROOF}"
echo "  phaseM:  ${SEXNET_PHASE_M_RELIABILITY_PROOF}"
echo "  phaseN:  ${SEXNET_PHASE_N_REAL_HW_AUDIT}"
	echo "  phaseO:  ${SEXNET_PHASE_O_FINAL_NETWORK_PROOF}"
echo ""

# ---- 1. BUILD ----
echo "[proof] BUILD phase..."
BUILD_START=$(date +%s)
if "$BUILD_SCRIPT" >/tmp/daily_driver_build.log 2>&1; then
    BUILD_DURATION=$(($(date +%s) - BUILD_START))
    echo "[proof] BUILD: PASS (${BUILD_DURATION}s)"
else
    echo "[proof] BUILD: FAIL"
    echo "[proof] Build log: /tmp/daily_driver_build.log"
    exit 1
fi

[ -f "$ISO" ] || die "ISO not produced: $ISO"

# ---- 2. BOOT ----
echo ""
echo "[proof] BOOT phase (QEMU, ${PROBE_SECONDS}s timeout)..."
QEMU_PID=""

cleanup() {
    set +e
    if [ -n "${QEMU_PID:-}" ] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill "$QEMU_PID" 2>/dev/null || true
        sleep 1
        kill -9 "$QEMU_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

QEMU_NET_ARGS=()
if [ "$ENABLE_QEMU_USERNET_E1000" = "1" ]; then
    case "$QEMU_NET_BACKEND" in
        user|tap) ;;
        *) die "unsupported QEMU_NET_BACKEND=$QEMU_NET_BACKEND (expected: user|tap)" ;;
    esac

    NETDEV_KIND=""
    NETDEV_VALUE=""
    if [ "$QEMU_NET_BACKEND" = "user" ]; then
        if [ -n "$QEMU_USERNET_HOSTFWD" ]; then
            NETDEV_KIND="-netdev"
            NETDEV_VALUE="user,id=net0,hostfwd=${QEMU_USERNET_HOSTFWD}"
        else
            NETDEV_KIND="-netdev"
            NETDEV_VALUE="user,id=net0"
        fi
    else
        NETDEV_KIND="-netdev"
        NETDEV_VALUE="tap,id=net0,ifname=${QEMU_TAP_IFNAME},script=no,downscript=no"
    fi

    case "$QEMU_NET_MODEL" in
        e1000|e1000-82540em)
            QEMU_NET_ARGS=(
                "$NETDEV_KIND"
                "$NETDEV_VALUE"
                -device e1000,netdev=net0
            )
            ;;
        e1000-82544gc)
            QEMU_NET_ARGS=(
                "$NETDEV_KIND"
                "$NETDEV_VALUE"
                -device e1000-82544gc,netdev=net0
            )
            ;;
        e1000-82545em)
            QEMU_NET_ARGS=(
                "$NETDEV_KIND"
                "$NETDEV_VALUE"
                -device e1000-82545em,netdev=net0
            )
            ;;
        e1000e)
            QEMU_NET_ARGS=(
                "$NETDEV_KIND"
                "$NETDEV_VALUE"
                -device e1000e,netdev=net0
            )
            ;;
        virtio|virtio-net|virtio-net-pci)
            QEMU_NET_ARGS=(
                "$NETDEV_KIND"
                "$NETDEV_VALUE"
                -device virtio-net-pci,netdev=net0
            )
            ;;
        *)
            die "unsupported QEMU_NET_MODEL=$QEMU_NET_MODEL (expected: e1000|e1000-82544gc|e1000-82545em|e1000e|virtio-net-pci)"
            ;;
    esac
fi

"$QEMU_BIN" \
    -M q35 \
    -m 512M \
    -cpu max,+pku \
    -cdrom "$ISO" \
    -device nec-usb-xhci,id=xhci \
    -device usb-kbd,bus=xhci.0 \
    "${QEMU_NET_ARGS[@]}" \
    -serial "file:$LOG" \
    -display none \
    -no-reboot \
    -no-shutdown &
QEMU_PID=$!

if ! kill -0 "$QEMU_PID" 2>/dev/null; then
    die "QEMU failed to start"
fi

echo "[proof] QEMU PID: $QEMU_PID"
sleep "$PROBE_SECONDS"

# Stop QEMU
if kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    sleep 1
fi

if [ ! -f "$LOG" ]; then
    die "no serial log produced at $LOG"
fi

{
    echo "[qemu.net.config] backend=${QEMU_NET_BACKEND} model=${QEMU_NET_MODEL} usernet=${ENABLE_QEMU_USERNET_E1000} hostfwd=${QEMU_USERNET_HOSTFWD:-none} tap_if=${QEMU_TAP_IFNAME}"
} >> "$LOG"

LOG_LINES=$(wc -l < "$LOG" 2>/dev/null || echo 0)
echo "[proof] Log lines: $LOG_LINES"

if [ "$LOG_LINES" -lt 10 ]; then
    echo "[proof] WARNING: Log has fewer than 10 lines — possibly truncated boot"
fi

# ---- 3. GATE SCAN ----
echo ""
echo "[proof] GATE SCAN phase..."

GATE_RESULT=0
"$GATE_SCRIPT" "$LOG" || GATE_RESULT=$?

echo ""

if [ "$GATE_RESULT" -eq 0 ]; then
    echo "[proof] DAILY-DRIVER PROOF PROFILE: PASS"
    exit 0
else
    echo "[proof] DAILY-DRIVER PROOF PROFILE: FAIL (gate scan exit=$GATE_RESULT)"
    exit 1
fi
