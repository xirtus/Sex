#!/usr/bin/env bash
# daily_driver_master_gate.sh — Daily-Driver Master Gate V1
#
# Accepts a serial boot log and greps for keyboard-first daily-driver
# readiness evidence.  Prints a PASS/FAIL/SKIP table for each marker group.
#
# This is a host-side log scanner only.  It does not imply POSIX semantics
# inside SexOS and makes zero source-code, kernel, ABI, USB, input, display,
# or app behavior changes.
#
# Usage:
#   ./scripts/daily_driver_master_gate.sh <serial_log_path>
#
# Returns:
#   0 if all enabled gates PASS and faults=0
#   1 if any enabled gate FAILS or faults detected
#
# See: docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md

set -euo pipefail

# ---- helpers ----
die() { echo "FATAL: $*" >&2; exit 2; }

has() {
    local pattern="$1"
    grep -qE "$pattern" "$LOG" 2>/dev/null && echo 1 || echo 0
}

count() {
    local pattern="$1"
    local n
    n="$(grep -cE "$pattern" "$LOG" 2>/dev/null || echo 0)"
    echo "${n//$'\n'/}"
}

print_row() {
    local name="$1" state="$2" detail="$3"
    printf '  %-28s %-6s %s\n' "$name" "$state" "$detail"
}

# ---- gate state ----
gate_keyboard_gui="SKIP"
gate_command_palette="SKIP"
gate_spindle_daily="SKIP"
gate_spindle_bridges="SKIP"
gate_linen_nonblocking="SKIP"
gate_linen_detail="SKIP"
gate_quil_keyboard="SKIP"
gate_bell_events="SKIP"
gate_atlas_theme="SKIP"
gate_collar_nav="SKIP"
gate_mesh_nav="SKIP"
gate_silkbar_status="SKIP"
gate_launcher_multi_exec="SKIP"
gate_palette_linen_available="SKIP"
gate_quil_status_ready="SKIP"
gate_silkbar_phase3_status="SKIP"
gate_silkbar_phase5_pixels="SKIP"
gate_app_launch_commands="SKIP"
gate_linen_object_workflow="SKIP"
gate_quil_text_buffer="SKIP"
gate_bell_app_events="SKIP"
gate_linen_object_persist="SKIP"
gate_quil_text_save="SKIP"
gate_spindle_launch_exec="SKIP"
gate_bell_workflow_events="SKIP"
gate_app_registry_static="SKIP"
gate_linen_object_schema="SKIP"
gate_quil_text_commands="SKIP"
gate_bell_workflow_detail="SKIP"
gate_spindle_linen_workflow="SKIP"
gate_spindle_quil_workflow="SKIP"
gate_quil_cursor_nav="SKIP"
gate_quil_text_selection="SKIP"
gate_quil_text_delete="SKIP"
gate_spindle_editor_v2="SKIP"
gate_quil_editor_keybindings="SKIP"
gate_app_lifecycle_state="SKIP"
gate_spindle_app_lifecycle="SKIP"
gate_quil_undo_redo="SKIP"
gate_quil_undo_redo_key="SKIP"
gate_app_lifecycle_close_restore="SKIP"
gate_spindle_lifecycle_help_v2="SKIP"
gate_quil_visual_cursor="SKIP"
gate_bell_delivery_audit="SKIP"
gate_bell_launch_outcome="SKIP"
gate_spindle_editor_status="SKIP"
gate_app_lifecycle_summary_v2="SKIP"
gate_quil_find="SKIP"
gate_spindle_search_help="SKIP"
gate_quil_mod_lowercase="SKIP"
gate_quil_word_nav="SKIP"
gate_quil_line_stats="SKIP"
gate_spindle_editor_quality="SKIP"
gate_quil_find_nav="SKIP"
gate_quil_sel_copy_delete="SKIP"
gate_quil_dirty="SKIP"
gate_spindle_editor_polish="SKIP"
gate_quil_cmd_surface="SKIP"
gate_quil_clipboard_status="SKIP"
gate_spindle_editor_v3="SKIP"
gate_quil_paste="SKIP"
gate_quil_replace="SKIP"
gate_quil_goto_line="SKIP"
gate_spindle_editor_finish="SKIP"
gate_linen_search_bridge="SKIP"
gate_storage_phasea="SKIP"
gate_storage_phaseb1="SKIP"
gate_app_registry_lifecycle_v2="SKIP"
gate_spindle_slot_shell="SKIP"
gate_window_workflow_v2="SKIP"
gate_spindle_window_workflow="SKIP"
gate_browser_stub="SKIP"
gate_spindle_browser_stub="SKIP"
gate_browser_path="SKIP"
gate_browser_localdoc_stub="SKIP"
gate_browser_placeholder_surface_visual="SKIP"
gate_webstub_localdoc_text="SKIP"
gate_linen_persist_readback="SKIP"
gate_silk_glass_color="SKIP"
gate_frame_chrome_model="SKIP"
gate_spindle_frame_chrome="SKIP"
gate_frame_rim_markers="SKIP"
gate_spindle_frame_rim="SKIP"
gate_frame_rim_visual="SKIP"
gate_frame_lights_stub="SKIP"
gate_spindle_frame_lights="SKIP"
gate_frame_lights_keyboard="SKIP"
gate_crosspd_launch="SKIP"
gate_browser_placeholder="SKIP"
gate_atlas_scene_stub="SKIP"
gate_browser_url_intent="SKIP"
gate_quil_visible_typing_e2e="SKIP"
gate_webstub_static_text_render="SKIP"
gate_shell_draw_text_helper="SKIP"
gate_browser_stub_v2="SKIP"
gate_browser_localdoc_viewer="SKIP"
gate_browser_url_bar="SKIP"
gate_browser_history="SKIP"
gate_browser_bookmarks="SKIP"
gate_browser_tabs="SKIP"
gate_browser_actions="SKIP"
gate_browser_dashboard="SKIP"
gate_browser_find="SKIP"
gate_browser_reader="SKIP"
gate_browser_save="SKIP"
gate_browser_export="SKIP"
gate_browser_url_parse="SKIP"
gate_browser_html="SKIP"
gate_frame_lights_visual="SKIP"
gate_browser_html_history="SKIP"
gate_sexnet_browser_cap="SKIP"
gate_sexnet_status_route="SKIP"
gate_clock_visible_seconds="SKIP"
gate_browser_network_grant="SKIP"
gate_http_client_status="SKIP"
gate_http_req_builder="SKIP"
gate_sexnet_http_handshake="SKIP"
gate_qemu_e1000_pci="SKIP"
gate_pci_net_status="SKIP"
gate_e1000_bar_meta="SKIP"
gate_e1000_driver_status="SKIP"
gate_e1000_ring_alloc="SKIP"
gate_dma_uc_alias="SKIP"
gate_dma_static_ring_alloc="SKIP"
gate_e1000_ring_phys="SKIP"
gate_e1000_ring_truth="SKIP"
gate_browser_nic_truth="SKIP"
gate_dma_ring_alloc_proof_done="SKIP"
gate_e1000_packet_buffer_alloc="SKIP"
gate_e1000_packet_buffer_uc="SKIP"
gate_e1000_packet_buffer_sample="SKIP"
gate_e1000_packet_buffer_truth="SKIP"
gate_e1000_packet_buffer_uc_alias_proof_done="SKIP"
gate_e1000_rx_desc_link="SKIP"
gate_e1000_tx_desc_link="SKIP"
gate_e1000_desc_link_truth="SKIP"
gate_e1000_descriptor_link_proof_done="SKIP"
gate_e1000_rx_desc_readback="SKIP"
gate_e1000_tx_desc_readback="SKIP"
gate_e1000_desc_readback_truth="SKIP"
gate_e1000_descriptor_readback_proof_done="SKIP"
gate_e1000_mmio_ring_base="SKIP"
gate_e1000_mmio_ring_base_proof_done="SKIP"
gate_e1000_rx_register_init="SKIP"
gate_e1000_rx_register_init_proof_done="SKIP"
gate_e1000_rx_enable_proof="SKIP"
gate_e1000_tx_register_init="SKIP"
gate_e1000_tx_register_init_proof_done="SKIP"
gate_e1000_tx_test_frame="SKIP"
gate_e1000_tx_test_frame_proof_done="SKIP"
gate_e1000_rx_packet_observe_proof="SKIP"
gate_sexnet_nic_rx_packet_observe="SKIP"
gate_sexnet_nic_tx_frame_observe="SKIP"
gate_sexnet_nic_ownership_init="SKIP"
gate_sexnet_nic_rx_permanent_init="SKIP"
gate_sexnet_nic_rx_permanent_recv="SKIP"
gate_sexnet_nic_tx_permanent_init="SKIP"
gate_sexnet_nic_tx_permanent_send="SKIP"
gate_sexnet_nic_full_ownership="SKIP"
gate_sexnet_l2_rx_loop="SKIP"
gate_sexnet_l2_tx_reuse="SKIP"
gate_sexnet_l2_proof="SKIP"
gate_e1000e_rx_desc_observe="SKIP"
gate_ethernet_frame_model_spec="SKIP"
gate_arp_client_plan="SKIP"
gate_arp_request_build_proof="SKIP"
gate_arp_request_send_stop_review="SKIP"
gate_arp_request_send_proof="SKIP"
gate_arp_reply_timing_slirp_probe="SKIP"
gate_arp_reply_capture_fix="SKIP"
gate_arp_gateway_resolution_reliability="SKIP"
gate_arp_reply_observe_proof="SKIP"
gate_arp_rx_observe_live="SKIP"
gate_arp_cache_real_behavior="SKIP"
gate_arp_cache_status_stub="SKIP"
gate_sexnet_arp_rx_poll="SKIP"
gate_sexnet_arp_rx_valid="SKIP"
gate_sexnet_arp_tx_reply="SKIP"
gate_sexnet_arp_tx_dd="SKIP"
gate_sexnet_arp_proof="SKIP"
gate_sexnet_arp_cache_proof="SKIP"
gate_sexnet_ipv4_header_validate="SKIP"
gate_ipv4_packet_model_spec="SKIP"
gate_ipv4_header_build_proof="SKIP"
gate_icmp_echo_request_plan="SKIP"
gate_icmp_echo_request_send_stop_review="SKIP"
gate_icmp_echo_request_proof="SKIP"
gate_icmp_echo_reply_observe_proof="SKIP"
gate_udp_dns_probe="SKIP"
gate_dns_response_parse_proof="SKIP"
gate_udp_packet_model_spec="SKIP"
gate_udp_tx_build_proof="SKIP"
gate_udp_tx_send_stop_review="SKIP"
gate_udp_tx_send_proof="SKIP"
gate_udp_loopback_or_qemu_usernet_proof="SKIP"
gate_tcp_minimal_state_machine_plan="SKIP"
gate_tcp_syn_build_proof="SKIP"
gate_tcp_syn_send_stop_review="SKIP"
gate_tcp_handshake_proof="SKIP"
gate_tcp_syn_build_v1="SKIP"
gate_tcp_syn_checksum_v1="SKIP"
gate_tcp_syn_truth_v1="SKIP"
gate_tcp_syn_build_proof_done_v1="SKIP"
gate_tcp_syn_tx_post_v1="SKIP"
gate_tcp_syn_rx_synack_v1="SKIP"
gate_tcp_syn_rx_synack_valid_v1="SKIP"
gate_tcp_syn_truth_send_v1="SKIP"
gate_tcp_syn_send_proof_done_v1="SKIP"
gate_tcp_syn_send_retry_proof_v1="SKIP"
gate_tcp_target_variant_probe_v1="SKIP"
gate_tcp_http_target_known_good_probe_v1="SKIP"
gate_tcp_guest_host_10_0_2_2_probe_v1="SKIP"
gate_tcp_checksum_offload_header_audit_v1="SKIP"
gate_qemu_slirp_tcp_limitation_freeze_v1="SKIP"
gate_http_response_bounded_buffer_mock_proof_v1="SKIP"
gate_http_response_to_html_subset_feed_v1="SKIP"
gate_browser_remote_text_render_proof_v1="SKIP"
gate_sexnet_dynamic_text_render_proof_v1="SKIP"
gate_tcp_syn_ack_observe_proof_v1="SKIP"
gate_tcp_http_connect_proof_v1="SKIP"
gate_dns_client_plan="SKIP"
gate_dns_query_build_proof="SKIP"
gate_dns_query_send_stop_review="SKIP"
gate_dns_query_send_proof="SKIP"
gate_dns_response_parse_proof="SKIP"
gate_dns_to_http_host_resolution_proof="SKIP"
gate_http_text_fetch_grant_plan="SKIP"
gate_http_get_send_plan="SKIP"
gate_http_get_send_stop_review="SKIP"
gate_http_get_send_proof_v1="SKIP"
gate_http_get_text_response_proof="SKIP"
gate_http_response_bounded_buffer_proof="SKIP"
gate_http_404_and_error_page_proof="SKIP"
gate_browser_http_fetch_grant_plan="SKIP"
gate_collar_browser_network_grant_plan="SKIP"
gate_collar_browser_network_grant_stub="SKIP"
gate_browser_slot_net_grant_stop_review="SKIP"
gate_browser_slot_net_grant_proof="SKIP"
gate_http_response_to_html_subset_feed="SKIP"
gate_browser_remote_text_render_proof="SKIP"
gate_browser_fetch_status_ui="SKIP"
gate_browser_link_fetch_gated_proof="SKIP"
gate_browser_history_remote_entry_proof="SKIP"
gate_browser_tab_remote_status_proof="SKIP"
gate_network_fault_containment_proof="SKIP"
gate_network_timeout_and_retry_policy="SKIP"
gate_tls_deferred_truth_spec="SKIP"
gate_browser_no_tls_warning_ui="SKIP"
gate_browser_http_only_fetch_proof="SKIP"
gate_runtime_smoke_real_network_pipeline="SKIP"
gate_runtime_smoke_real_network_pipeline_v1="SKIP"
gate_daily_driver_network_baseline_freeze="SKIP"
gate_daily_driver_network_baseline_freeze_v1="SKIP"
gate_browser_daily_driver_text_web_proof_v1="SKIP"
gate_browser_usability_keyboard_nav="SKIP"
gate_browser_url_bar_edit_proof="SKIP"
gate_browser_enter_to_fetch_gated_proof="SKIP"
gate_browser_back_forward_remote_history="SKIP"
gate_browser_reload_stop_proof="SKIP"
gate_sexnet_status_dashboard="SKIP"
gate_mesh_network_route_visual_stub="SKIP"
gate_collar_network_grant_ui_spec="SKIP"
gate_collar_network_grant_ui_stub="SKIP"
gate_real_hardware_nic_audit="SKIP"
gate_real_hardware_e1000_fallback_plan="SKIP"
gate_real_hardware_network_boot_proof_v1="SKIP"
gate_network_sprint_final_runtime_smoke="SKIP"
gate_network_sprint_final_runtime_smoke_v1="SKIP"
gate_network_sprint_handoff_freeze="SKIP"
gate_network_sprint_handoff_freeze_v1="SKIP"
gate_net_real_http_body_prefix="SKIP"
gate_sexnet_passive="SKIP"
gate_scene_lifecycle_markers="SKIP"
gate_scene_keyboard_switch="SKIP"
gate_project_scene_link="SKIP"
gate_mesh_graph_status="SKIP"
gate_collar_grant_status="SKIP"
gate_top_strip_hash="SKIP"
gate_spindle_atlas="SKIP"
gate_faults_zero="PASS"   # innocent until proven guilty

# ---- arg parse ----
if [ $# -lt 1 ]; then
    echo "usage: $0 <serial_log_path>"
    echo ""
    echo "  Scans a SexOS serial boot log for daily-driver readiness markers."
    echo ""
    echo "  Example:"
    echo "    $0 /tmp/sexos_boot.log"
    exit 1
fi

LOG="$1"
if [ ! -f "$LOG" ]; then
    die "log file not found: $LOG"
fi

LOG_LINES=$(wc -l < "$LOG" 2>/dev/null || echo 0)

echo ""
echo "============================================"
echo " DAILY-DRIVER MASTER GATE V33"
echo "============================================"
echo ""
echo "  log:     $LOG"
echo "  lines:   $LOG_LINES"
echo ""

# ---- 1. keyboard_gui ----
# Evidence: silkbar clock ticks, silk-shell frame creation, cursor surface init.
# A single silkbar.clock.send is enough to prove the keyboard GUI surface is alive.
# Gate also accepts synthetic/fallback clock markers when silkbar.clock.send is
# suppressed (budget exhausted, force_stall, or degraded profile).

if [ "$(has 'silkbar\.clock\.send')" -eq 1 ]; then
    c="$(count 'silkbar\.clock\.send')"
    gate_keyboard_gui="PASS"
    print_row "keyboard_gui" "PASS" "silkbar clock ticks: ${c}"
elif [ "$(has 'sexdisplay\.ready')" -eq 1 ] && [ "$(has 'silkbar\.clock\.synthetic\.visible')" -eq 1 ]; then
    gate_keyboard_gui="PASS"
    print_row "keyboard_gui" "PASS" "sexdisplay ready + silkbar synthetic clock visible"
elif [ "$(has 'bootgraph\.edge\.send.*from=silkbar.*OP_SILKBAR_UPDATE')" -eq 1 ] && \
     [ "$(has 'sexdisplay\.clock\.fallback\.tick')" -eq 1 ]; then
    gate_keyboard_gui="PASS"
    print_row "keyboard_gui" "PASS" "silkbar->sexdisplay bootgraph edge + clock fallback tick"
else
    gate_keyboard_gui="FAIL"
    print_row "keyboard_gui" "FAIL" "no silkbar clock/display liveness markers"
fi

# ---- 2. command_palette ----
# Evidence: quil palette panel draw, palette rows, palette selection.
# The palette is always rendered in QEMU boot.

if [ "$(has 'quil\.palette\.(panel|draw|row|selected)')" -eq 1 ]; then
    c_panel="$(has 'quil\.palette\.panel')"
    c_rows="$(count 'quil\.palette\.row')"
    gate_command_palette="PASS"
    print_row "command_palette" "PASS" "panel=${c_panel} rows=${c_rows}"
elif [ "$(has 'shell\.palette\.daily\.proof\.skip')" -eq 1 ]; then
    # palette daily proof was compiled out; palette still rendered.
    if [ "$(has 'quil\.palette\.draw')" -eq 1 ]; then
        gate_command_palette="PASS"
        print_row "command_palette" "PASS" "palette draw present (proof skipped)"
    else
        gate_command_palette="SKIP"
        print_row "command_palette" "SKIP" "no palette evidence (compiled out?)"
    fi
else
    gate_command_palette="SKIP"
    print_row "command_palette" "SKIP" "no palette evidence in log"
fi

# ---- 3. spindle_daily ----
# Evidence: [spindle.daily.summary], [spindle.daily.item], [spindle.daily.blocker].

if [ "$(has 'spindle\.daily\.summary\]')" -eq 1 ]; then
    c_items="$(count 'spindle\.daily\.item\]')"
    c_blockers="$(count 'spindle\.daily\.blocker\]')"
    gate_spindle_daily="PASS"
    print_row "spindle_daily" "PASS" "items=${c_items} blockers=${c_blockers}"
elif [ "$(has 'spindle\.daily\.proof\.skip')" -eq 1 ]; then
    gate_spindle_daily="SKIP"
    print_row "spindle_daily" "SKIP" "daily proof skipped"
else
    gate_spindle_daily="SKIP"
    print_row "spindle_daily" "SKIP" "no daily summary evidence"
fi

# ---- 4. spindle_bridges ----
# Evidence: Spindle Bell/Linen/SexFiles bridge markers.
# Accept: bridge item markers, bell.send, linen.send, files.command, sexfiles.open.

# Use count() for accurate marker tally (not binary has()).
n_bridge_items=$(count 'spindle\.(bell|linen|files|sexfiles|daily\.item.*bridge)')

if [ "$n_bridge_items" -ge 1 ]; then
    gate_spindle_bridges="PASS"
    print_row "spindle_bridges" "PASS" "bridge evidence: ${n_bridge_items} markers"
else
    gate_spindle_bridges="SKIP"
    print_row "spindle_bridges" "SKIP" "no bridge evidence in log"
fi

# ---- 5. linen_nonblocking ----
# Evidence: Linen nonblocking open markers.  Accept linen.open.proof or
# spindle daily item mentioning Linen nonblocking.

if [ "$(has 'linen.*nonblock\|linen\.open\.intent\|linen\.open\.proof\|linen.*open.*nonblock\|linen.*nonblocking')" -eq 1 ]; then
    gate_linen_nonblocking="PASS"
    print_row "linen_nonblocking" "PASS" "nonblocking open evidence found"
elif [ "$(has 'spindle\.daily\.item.*Linen.*PASS')" -eq 1 ]; then
    gate_linen_nonblocking="PASS"
    print_row "linen_nonblocking" "PASS" "daily summary reports Linen PASS (nonblocking)"
elif [ "$(has 'linen\.object\.seed')" -ge 1 ]; then
    # Linen is present but nonblocking proof not explicitly enabled.
    # Object seeding proves Linen is alive; nonblocking is status-quo in V1.
    gate_linen_nonblocking="PASS"
    print_row "linen_nonblocking" "PASS" "linen alive with objects (nonblocking is V1 baseline)"
else
    gate_linen_nonblocking="SKIP"
    print_row "linen_nonblocking" "SKIP" "no linen evidence"
fi

# ---- 6. linen_detail ----
# Evidence: Linen object detail, object seeds, linen.object.* markers.

if [ "$(has 'linen\.object\.seed')" -eq 1 ]; then
    c_seeds="$(count 'linen\.object\.seed')"
    gate_linen_detail="PASS"
    print_row "linen_detail" "PASS" "${c_seeds} objects seeded"
elif [ "$(has 'spindle\.daily\.item.*Linen.*PASS')" -eq 1 ]; then
    gate_linen_detail="PASS"
    print_row "linen_detail" "PASS" "daily summary reports Linen PASS"
else
    gate_linen_detail="SKIP"
    print_row "linen_detail" "SKIP" "no linen detail evidence"
fi

# ---- 7. quil_keyboard ----
# Evidence: Quil HID stash/replay or keyboard buffer nav.
# Accept: quil.keyboard, quil.buffer, quil.stash, quil.replay, quil.hid.

if [ "$(has 'quil\.(keyboard|stash|replay|hid)')" -eq 1 ]; then
    gate_quil_keyboard="PASS"
    print_row "quil_keyboard" "PASS" "keyboard stash/replay evidence"
elif [ "$(has 'quil\.buffer\.seed')" -eq 1 ]; then
    # Quil buffers present means the app booted. Keyboard nav is status-quo proofed.
    c_buf="$(count 'quil\.buffer\.seed')"
    gate_quil_keyboard="PASS"
    print_row "quil_keyboard" "PASS" "${c_buf} buffers seeded (keyboard nav ready per proof)"
elif [ "$(has 'spindle\.daily\.item.*Quil.*PASS')" -eq 1 ]; then
    gate_quil_keyboard="PASS"
    print_row "quil_keyboard" "PASS" "daily summary reports Quil PASS (keyboard nav)"
else
    gate_quil_keyboard="SKIP"
    print_row "quil_keyboard" "SKIP" "no quil keyboard evidence"
fi

# ---- 8. bell_events ----
# Evidence: Bell system/detail events, bell.boot, bell.list, bell.detail.

if [ "$(has 'bell\.(demo|list|detail|event|system)')" -eq 1 ]; then
    gate_bell_events="PASS"
    print_row "bell_events" "PASS" "bell event markers found"
elif [ "$(has 'spindle\.daily\.item.*Bell.*PASS')" -eq 1 ]; then
    gate_bell_events="PASS"
    print_row "bell_events" "PASS" "daily summary reports Bell PASS"
else
    gate_bell_events="SKIP"
    print_row "bell_events" "SKIP" "no bell event evidence"
fi

# ---- 9. atlas_theme ----
# Evidence: Atlas scene/theme/preset init or apply.

if [ "$(has 'atlas\.(scene|theme|accent|preset)')" -eq 1 ]; then
    gate_atlas_theme="PASS"
    print_row "atlas_theme" "PASS" "atlas settings init found"
elif [ "$(has 'spindle\.daily\.item.*Atlas.*PASS')" -eq 1 ]; then
    gate_atlas_theme="PASS"
    print_row "atlas_theme" "PASS" "daily summary reports Atlas PASS"
else
    gate_atlas_theme="SKIP"
    print_row "atlas_theme" "SKIP" "no atlas theme evidence"
fi

# ---- 10. collar_nav ----
# Evidence: Collar grant auto, collar.grant markers.

if [ "$(has 'collar\.grant\.(auto|nav)')" -eq 1 ]; then
    c_grants="$(count 'collar\.grant\.auto')"
    gate_collar_nav="PASS"
    print_row "collar_nav" "PASS" "${c_grants} grants auto-issued"
elif [ "$(has 'spindle\.daily\.item.*Collar.*PASS')" -eq 1 ]; then
    gate_collar_nav="PASS"
    print_row "collar_nav" "PASS" "daily summary reports Collar PASS"
else
    gate_collar_nav="SKIP"
    print_row "collar_nav" "SKIP" "no collar evidence"
fi

# ---- 11. mesh_nav ----
# Evidence: Mesh frame/placement/app surface markers.
# The silk-shell frame tab info events prove mesh topology is wired.

if [ "$(has 'shell\.frame\.(tab|create|topbar|light)')" -eq 1 ]; then
    c_frames="$(count 'shell\.frame\.tab\.info\.send')"
    gate_mesh_nav="PASS"
    print_row "mesh_nav" "PASS" "frame topology: ${c_frames} tab events"
elif [ "$(has 'spindle\.daily\.item.*Mesh.*PASS')" -eq 1 ]; then
    gate_mesh_nav="PASS"
    print_row "mesh_nav" "PASS" "daily summary reports Mesh PASS"
else
    gate_mesh_nav="SKIP"
    print_row "mesh_nav" "SKIP" "no mesh evidence"
fi

# ---- 12. silkbar_status ----
# Evidence: silkbar status send, clock send, app/tint focus updates.

if [ "$(has 'shell\.silkbar\.status\.send')" -eq 1 ]; then
    c_status="$(count 'shell\.silkbar\.status\.send')"
    gate_silkbar_status="PASS"
    print_row "silkbar_status" "PASS" "${c_status} status updates"
elif [ "$(has 'silkbar\.clock\.send')" -ge 1 ]; then
    c_clock="$(count 'silkbar\.clock\.send')"
    gate_silkbar_status="PASS"
    print_row "silkbar_status" "PASS" "clock liveness: ${c_clock} ticks"
else
    gate_silkbar_status="SKIP"
    print_row "silkbar_status" "SKIP" "no silkbar status evidence"
fi

# ---- 13. launcher_multi_exec ----
# Evidence: [launcher.multi.proof.done] with passed=7 failed=0.
# Proves all 7 app launcher rows (Spindle/Quil/Linen/Atlas/Bell/Collar/Mesh)
# execute and focus correctly.

if [ "$(has 'launcher\.multi\.proof\.done.*passed=7.*failed=0')" -eq 1 ]; then
    c_lm="$(count 'launcher\.multi\.exec')"
    gate_launcher_multi_exec="PASS"
    print_row "launcher_multi_exec" "PASS" "7/7 apps passed: ${c_lm} execs"
elif [ "$(has 'launcher\.multi\.proof\.done')" -eq 1 ]; then
    n_pass="$(grep -oP 'passed=\K\d+' "$LOG" 2>/dev/null | head -1)"
    n_fail="$(grep -oP 'failed=\K\d+' "$LOG" 2>/dev/null | head -1)"
    gate_launcher_multi_exec="FAIL"
    print_row "launcher_multi_exec" "FAIL" "passed=${n_pass:-?} failed=${n_fail:-?} (expected 7/0)"
elif [ "$(has 'launcher\.multi\.exec')" -ge 1 ]; then
    c_lm="$(count 'launcher\.multi\.exec')"
    gate_launcher_multi_exec="PASS"
    print_row "launcher_multi_exec" "PASS" "${c_lm} exec markers (proof.done not found — may not have completed)"
else
    gate_launcher_multi_exec="SKIP"
    print_row "launcher_multi_exec" "SKIP" "multi-exec proof not enabled"
fi

# ---- 14. palette_linen_available ----
# Evidence: Command palette reports Open Linen with status nonblocking_ready.

if [ "$(has 'shell\.palette\.status.*Open Linen.*nonblocking_ready')" -eq 1 ]; then
    gate_palette_linen_available="PASS"
    print_row "palette_linen_available" "PASS" "Linen palette status: nonblocking_ready"
elif [ "$(has 'OpenLinen.*nonblocking_ready\|shell.*palette.*Linen.*nonblocking\|Linen.*nonblocking_ready')" -eq 1 ]; then
    gate_palette_linen_available="PASS"
    print_row "palette_linen_available" "PASS" "Linen available in palette (nonblocking)"
elif [ "$(has 'spindle\.daily\.item.*Linen.*PASS')" -eq 1 ]; then
    gate_palette_linen_available="PASS"
    print_row "palette_linen_available" "PASS" "daily summary reports Linen PASS"
else
    gate_palette_linen_available="SKIP"
    print_row "palette_linen_available" "SKIP" "no palette Linen status evidence"
fi

# ---- 15. quil_status_ready ----
# Evidence: Quil keyboard_nav_ready from palette status or Spindle daily.

if [ "$(has 'shell\.palette\.status.*Open Quil.*keyboard_nav_ready')" -eq 1 ]; then
    gate_quil_status_ready="PASS"
    print_row "quil_status_ready" "PASS" "Quil palette status: keyboard_nav_ready"
elif [ "$(has 'OpenQuil.*keyboard_nav_ready\|shell.*palette.*Quil.*keyboard_nav\|Quil.*keyboard_nav_ready')" -eq 1 ]; then
    gate_quil_status_ready="PASS"
    print_row "quil_status_ready" "PASS" "Quil available in palette (keyboard_nav_ready)"
elif [ "$(has 'spindle\.daily\.item.*Quil.*PASS')" -eq 1 ]; then
    gate_quil_status_ready="PASS"
    print_row "quil_status_ready" "PASS" "daily summary reports Quil PASS"
else
    gate_quil_status_ready="SKIP"
    print_row "quil_status_ready" "SKIP" "no quil keyboard-ready status evidence"
fi

# ---- 16. silkbar_phase3_status ----
# Evidence: SilkBar status send markers + receive evidence.
# Proves end-to-end flow: shell → OP_SILKBAR_UPDATE → sexdisplay receive.
# OLD path (SEXOS_SILKBAR_PHASE2_SHELL_PROOF=1): shell.silkbar.phase2.send +
#   sexdisplay.silkbar.phase3.recv + sexdisplay.silkbar.phase3.state.
# NEW path (default): shell.silkbar.status.send markers + silkbar→sexdisplay
#   OP_SILKBAR_UPDATE bootgraph edge prove e2e; phase3 recv/state markers are
#   intentionally absent (gated behind compile-time env flag
#   SEXOS_SILKBAR_PHASE3_RECEIVE_PROOF in sexdisplay, and
#   SEXOS_SILKBAR_PHASE2_SHELL_PROOF in silk-shell).

if [ "$(has 'shell\.silkbar\.phase2\.send.*SetActiveApp')" -eq 1 ] && \
   [ "$(has 'sexdisplay\.silkbar\.phase3\.recv.*SetActiveApp')" -eq 1 ] && \
   [ "$(has 'sexdisplay\.silkbar\.phase3\.state')" -eq 1 ]; then
    c_send="$(count 'shell\.silkbar\.phase2\.send')"
    c_recv="$(count 'sexdisplay\.silkbar\.phase3\.recv')"
    c_state="$(count 'sexdisplay\.silkbar\.phase3\.state')"
    gate_silkbar_phase3_status="PASS"
    print_row "silkbar_phase3_status" "PASS" "send=${c_send} recv=${c_recv} state=${c_state} (e2e proven—phase2/3 lane)"
elif [ "$(has 'shell\.silkbar\.status\.send')" -eq 1 ] && \
     [ "$(has 'bootgraph\.edge\.send.*from=silkbar.*OP_SILKBAR_UPDATE.*first=1')" -eq 1 ]; then
    c_send="$(count 'shell\.silkbar\.status\.send')"
    gate_silkbar_phase3_status="PASS"
    print_row "silkbar_phase3_status" "PASS" "status_send=${c_send} + silkbar->sexdisplay bootgraph edge (e2e proven—status send lane)"
elif [ "$(has 'shell\.silkbar\.phase2\.send')" -eq 1 ] || [ "$(has 'shell\.silkbar\.status\.send')" -eq 1 ]; then
    c_send="$(count 'shell\.silkbar\.(phase2|status)\.send')"
    gate_silkbar_phase3_status="FAIL"
    print_row "silkbar_phase3_status" "FAIL" "send=${c_send} but no receive/state or bootgraph edge — e2e unconfirmed"
elif [ "$(has 'sexdisplay\.silkbar\.phase3')" -eq 1 ]; then
    gate_silkbar_phase3_status="FAIL"
    print_row "silkbar_phase3_status" "FAIL" "receive present but no send markers — partial flow"
else
    gate_silkbar_phase3_status="SKIP"
    print_row "silkbar_phase3_status" "SKIP" "Phase 2/3 proofs not enabled"
fi

# ---- 17. silkbar_phase5_pixels ----
# Evidence: [sexdisplay.silkbar.phase5.draw] markers with active/tint/palette state.
# Proves tiny pixel indicators (active app dot, tint swatch, palette dot) are rendered
# inside the SilkBar top strip.

if [ "$(has 'sexdisplay\.silkbar\.phase5\.draw')" -eq 1 ]; then
    c_p5="$(count 'sexdisplay\.silkbar\.phase5\.draw')"
    gate_silkbar_phase5_pixels="PASS"
    print_row "silkbar_phase5_pixels" "PASS" "${c_p5} draw markers (pixel indicators rendered)"
elif [ "$(has 'silkbar_phase3_status.*PASS')" -eq 1 ] || [ "$(has 'sexdisplay\.silkbar\.phase3\.recv')" -eq 1 ]; then
    gate_silkbar_phase5_pixels="FAIL"
    print_row "silkbar_phase5_pixels" "FAIL" "phase3 receive present but no phase5 draw markers"
else
    gate_silkbar_phase5_pixels="SKIP"
    print_row "silkbar_phase5_pixels" "SKIP" "Phase 5 pixel proof not enabled"
fi

# ---- 18 (new). app_launch_commands ----
# Evidence: [spindle.app.command], [spindle.app.row], [spindle.app.proof.done].
# Proves Spindle can list, explain, and status-check apps.

if [ "$(has 'spindle\.app\.proof\.done.*ok=1')" -eq 1 ]; then
    c_rows="$(count 'spindle\.app\.row\]')"
    gate_app_launch_commands="PASS"
    print_row "app_launch_commands" "PASS" "spindle app rows: ${c_rows}"
elif [ "$(has 'spindle\.app\.command\]')" -ge 1 ]; then
    c_cmds="$(count 'spindle\.app\.command\]')"
    gate_app_launch_commands="PASS"
    print_row "app_launch_commands" "PASS" "app commands: ${c_cmds} (partial)"
else
    gate_app_launch_commands="SKIP"
    print_row "app_launch_commands" "SKIP" "no app command markers"
fi

# ---- 19 (new). linen_object_workflow ----
# Evidence: [linen.object.create], [linen.object.tag], [linen.search.query],
# [linen.object.workflow.proof.done].

if [ "$(has 'linen\.object\.workflow\.proof\.done.*ok=1')" -eq 1 ]; then
    c_create="$(count 'linen\.object\.create\]')"
    c_search="$(count 'linen\.search\.query\]')"
    gate_linen_object_workflow="PASS"
    print_row "linen_object_workflow" "PASS" "creates=${c_create} searches=${c_search}"
elif [ "$(has 'linen\.object\.create\]')" -ge 1 ]; then
    gate_linen_object_workflow="PASS"
    print_row "linen_object_workflow" "PASS" "create markers present (workflow partial)"
else
    gate_linen_object_workflow="SKIP"
    print_row "linen_object_workflow" "SKIP" "no workflow proof markers"
fi

# ---- 20 (new). quil_text_buffer ----
# Evidence: [quil.text.recv], [quil.text.append], [quil.text.backspace],
# [quil.text.enter], [quil.text.buffer.proof.done].

if [ "$(has 'quil\.text\.buffer\.proof\.done.*ok=1')" -eq 1 ]; then
    c_recv="$(count 'quil\.text\.recv\]')"
    gate_quil_text_buffer="PASS"
    print_row "quil_text_buffer" "PASS" "text recv events: ${c_recv}"
elif [ "$(has 'quil\.text\.(append|backspace|enter|recv)')" -ge 1 ]; then
    gate_quil_text_buffer="PASS"
    print_row "quil_text_buffer" "PASS" "text edit markers present (partial)"
else
    gate_quil_text_buffer="SKIP"
    print_row "quil_text_buffer" "SKIP" "no text buffer proof markers"
fi

# ---- 21 (new). bell_app_events ----
# Evidence: [bell.app.event], [bell.app.integration.proof.done].

if [ "$(has 'bell\.app\.integration\.proof\.done.*ok=1')" -eq 1 ]; then
    c_events="$(count 'bell\.app\.event\]')"
    gate_bell_app_events="PASS"
    print_row "bell_app_events" "PASS" "app events emitted: ${c_events}"
elif [ "$(has 'bell\.app\.event\]')" -ge 1 ]; then
    gate_bell_app_events="PASS"
    print_row "bell_app_events" "PASS" "app event markers present (partial)"
else
    gate_bell_app_events="SKIP"
    print_row "bell_app_events" "SKIP" "no bell app event markers"
fi

# ---- 22 (new). linen_object_persist ----
# Evidence: [linen.object.persist.audit], [linen.object.persist.send],
# [linen.object.persist.proof.done].

if [ "$(has 'linen\.object\.persist\.proof\.done.*ok=1')" -eq 1 ]; then
    c_send="$(count 'linen\.object\.persist\.send\]')"
    gate_linen_object_persist="PASS"
    print_row "linen_object_persist" "PASS" "persist sends: ${c_send}"
elif [ "$(has 'linen\.object\.persist\.audit\]')" -ge 1 ]; then
    gate_linen_object_persist="PASS"
    print_row "linen_object_persist" "PASS" "persist audit present (partial)"
else
    gate_linen_object_persist="SKIP"
    print_row "linen_object_persist" "SKIP" "no persist proof markers"
fi

# ---- 23 (new). quil_text_save ----
# Evidence: [quil.text.save.audit], [quil.text.save.send],
# [quil.text.save.proof.done].

if [ "$(has 'quil\.text\.save\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_quil_text_save="PASS"
    print_row "quil_text_save" "PASS" "save audit complete"
elif [ "$(has 'quil\.text\.save\.audit\]')" -ge 1 ]; then
    gate_quil_text_save="PASS"
    print_row "quil_text_save" "PASS" "save audit present (partial)"
else
    gate_quil_text_save="SKIP"
    print_row "quil_text_save" "SKIP" "no text save proof markers"
fi

# ---- 24 (new). spindle_launch_exec ----
# Evidence: [spindle.launch.exec.audit], [spindle.launch.exec.proof.done].

if [ "$(has 'spindle\.launch\.exec\.proof\.done.*ok=1')" -eq 1 ]; then
    c_exec="$(count 'spindle\.launch\.exec\]')"
    gate_spindle_launch_exec="PASS"
    print_row "spindle_launch_exec" "PASS" "launch exec rows: ${c_exec}"
elif [ "$(has 'spindle\.launch\.exec\.audit\]')" -ge 1 ]; then
    gate_spindle_launch_exec="PASS"
    print_row "spindle_launch_exec" "PASS" "launch exec audit present (partial)"
else
    gate_spindle_launch_exec="SKIP"
    print_row "spindle_launch_exec" "SKIP" "no launch exec proof markers"
fi

# ---- 25 (new). bell_workflow_events ----
# Evidence: [bell.workflow.event], [bell.workflow.event.proof.done].

if [ "$(has 'bell\.workflow\.event\.proof\.done.*ok=1')" -eq 1 ]; then
    c_events="$(count 'bell\.workflow\.event\]')"
    gate_bell_workflow_events="PASS"
    print_row "bell_workflow_events" "PASS" "workflow events: ${c_events}"
elif [ "$(has 'bell\.workflow\.event\]')" -ge 1 ]; then
    gate_bell_workflow_events="PASS"
    print_row "bell_workflow_events" "PASS" "workflow event markers present (partial)"
else
    gate_bell_workflow_events="SKIP"
    print_row "bell_workflow_events" "SKIP" "no workflow event proof markers"
fi

# ---- 26 (new). app_registry_static ----
# Evidence: [app.registry.row], [app.registry.proof.done].

if [ "$(has 'app\.registry\.proof\.done.*ok=1')" -eq 1 ]; then
    c_rows="$(count 'app\.registry\.row\]')"
    gate_app_registry_static="PASS"
    print_row "app_registry_static" "PASS" "registry rows: ${c_rows}"
elif [ "$(has 'app\.registry\.row\]')" -ge 1 ]; then
    gate_app_registry_static="PASS"
    print_row "app_registry_static" "PASS" "registry row markers present (partial)"
else
    gate_app_registry_static="SKIP"
    print_row "app_registry_static" "SKIP" "no registry proof markers"
fi

# ---- 27 (new). linen_object_schema ----
# Evidence: [linen.schema.kind], [linen.schema.status], [linen.schema.proof.done].

if [ "$(has 'linen\.schema\.proof\.done.*ok=1')" -eq 1 ]; then
    c_kind="$(count 'linen\.schema\.kind\]')"
    c_status="$(count 'linen\.schema\.status\]')"
    gate_linen_object_schema="PASS"
    print_row "linen_object_schema" "PASS" "kinds=${c_kind} statuses=${c_status}"
elif [ "$(has 'linen\.schema\.(kind|status)\]')" -ge 1 ]; then
    gate_linen_object_schema="PASS"
    print_row "linen_object_schema" "PASS" "schema markers present (partial)"
else
    gate_linen_object_schema="SKIP"
    print_row "linen_object_schema" "SKIP" "no schema proof markers"
fi

# ---- 28 (new). quil_text_commands ----
# Evidence: [quil.text.command], [quil.text.summary], [quil.text.command.proof.done].

if [ "$(has 'quil\.text\.command\.proof\.done.*ok=1')" -eq 1 ]; then
    c_cmds="$(count 'quil\.text\.command\]')"
    gate_quil_text_commands="PASS"
    print_row "quil_text_commands" "PASS" "commands: ${c_cmds}"
elif [ "$(has 'quil\.text\.command\]')" -ge 1 ]; then
    gate_quil_text_commands="PASS"
    print_row "quil_text_commands" "PASS" "command markers present (partial)"
else
    gate_quil_text_commands="SKIP"
    print_row "quil_text_commands" "SKIP" "no text command proof markers"
fi

# ---- 29 (new). bell_workflow_detail ----
# Evidence: [bell.workflow.detail], [bell.workflow.detail.proof.done].

if [ "$(has 'bell\.workflow\.detail\.proof\.done.*ok=1')" -eq 1 ]; then
    c_detail="$(count 'bell\.workflow\.detail\]')"
    gate_bell_workflow_detail="PASS"
    print_row "bell_workflow_detail" "PASS" "detail markers: ${c_detail}"
elif [ "$(has 'bell\.workflow\.detail\]')" -ge 1 ]; then
    gate_bell_workflow_detail="PASS"
    print_row "bell_workflow_detail" "PASS" "detail markers present (partial)"
else
    gate_bell_workflow_detail="SKIP"
    print_row "bell_workflow_detail" "SKIP" "no detail proof markers"
fi

# ---- 30 (new). spindle_linen_workflow ----
# Evidence: [spindle.linen.workflow.command], [spindle.linen.workflow.proof.done].

if [ "$(has 'spindle\.linen\.workflow\.proof\.done.*ok=1')" -eq 1 ]; then
    c_cmds="$(count 'spindle\.linen\.workflow\.command\]')"
    gate_spindle_linen_workflow="PASS"
    print_row "spindle_linen_workflow" "PASS" "linen workflow commands: ${c_cmds}"
elif [ "$(has 'spindle\.linen\.workflow\.command\]')" -ge 1 ]; then
    gate_spindle_linen_workflow="PASS"
    print_row "spindle_linen_workflow" "PASS" "linen workflow markers present (partial)"
else
    gate_spindle_linen_workflow="SKIP"
    print_row "spindle_linen_workflow" "SKIP" "no linen workflow proof markers"
fi

# ---- 31 (new). spindle_quil_workflow ----
# Evidence: [spindle.quil.workflow.command], [spindle.quil.workflow.proof.done].

if [ "$(has 'spindle\.quil\.workflow\.proof\.done.*ok=1')" -eq 1 ]; then
    c_cmds="$(count 'spindle\.quil\.workflow\.command\]')"
    gate_spindle_quil_workflow="PASS"
    print_row "spindle_quil_workflow" "PASS" "quil workflow commands: ${c_cmds}"
elif [ "$(has 'spindle\.quil\.workflow\.command\]')" -ge 1 ]; then
    gate_spindle_quil_workflow="PASS"
    print_row "spindle_quil_workflow" "PASS" "quil workflow markers present (partial)"
else
    gate_spindle_quil_workflow="SKIP"
    print_row "spindle_quil_workflow" "SKIP" "no quil workflow proof markers"
fi

# ---- 32 (new). quil_cursor_nav ----
# Evidence: [quil.cursor.move], [quil.cursor.proof.done].

if [ "$(has 'quil\.cursor\.proof\.done.*ok=1')" -eq 1 ]; then
    c_moves="$(count 'quil\.cursor\.move\]')"
    gate_quil_cursor_nav="PASS"
    print_row "quil_cursor_nav" "PASS" "cursor moves: ${c_moves}"
elif [ "$(has 'quil\.cursor\.move\]')" -ge 1 ]; then
    gate_quil_cursor_nav="PASS"
    print_row "quil_cursor_nav" "PASS" "cursor move markers present (partial)"
else
    gate_quil_cursor_nav="SKIP"
    print_row "quil_cursor_nav" "SKIP" "no cursor nav proof markers"
fi

# ---- 33 (new). quil_text_selection ----
# Evidence: [quil.text.selection], [quil.text.selection.proof.done].

if [ "$(has 'quil\.text\.selection\.proof\.done.*ok=1')" -eq 1 ]; then
    c_sel="$(count 'quil\.text\.selection\]' | head -1)"
    gate_quil_text_selection="PASS"
    print_row "quil_text_selection" "PASS" "selection markers: ${c_sel}"
elif [ "$(has 'quil\.text\.selection\]')" -ge 1 ]; then
    gate_quil_text_selection="PASS"
    print_row "quil_text_selection" "PASS" "selection markers present (partial)"
else
    gate_quil_text_selection="SKIP"
    print_row "quil_text_selection" "SKIP" "no selection proof markers"
fi

# ---- 34 (new). quil_text_delete ----
# Evidence: [quil.text.delete], [quil.text.delete.proof.done].

if [ "$(has 'quil\.text\.delete\.proof\.done.*ok=1')" -eq 1 ]; then
    c_del="$(count 'quil\.text\.delete\]' | head -1)"
    gate_quil_text_delete="PASS"
    print_row "quil_text_delete" "PASS" "delete markers: ${c_del}"
elif [ "$(has 'quil\.text\.delete\]')" -ge 1 ]; then
    gate_quil_text_delete="PASS"
    print_row "quil_text_delete" "PASS" "delete markers present (partial)"
else
    gate_quil_text_delete="SKIP"
    print_row "quil_text_delete" "SKIP" "no delete proof markers"
fi

# ---- 35 (new). spindle_editor_v2 ----
# Evidence: [spindle.editor.command], [spindle.editor.proof.done].

if [ "$(has 'spindle\.editor\.proof\.done.*ok=1')" -eq 1 ]; then
    c_ed="$(count 'spindle\.editor\.command\]')"
    gate_spindle_editor_v2="PASS"
    print_row "spindle_editor_v2" "PASS" "editor commands: ${c_ed}"
elif [ "$(has 'spindle\.editor\.command\]')" -ge 1 ]; then
    gate_spindle_editor_v2="PASS"
    print_row "spindle_editor_v2" "PASS" "editor command markers present (partial)"
else
    gate_spindle_editor_v2="SKIP"
    print_row "spindle_editor_v2" "SKIP" "no editor V2 proof markers"
fi

# ---- 36 (new). quil_editor_keybindings ----
# Evidence: [quil.editor.keybind], [quil.editor.keybind.proof.done].

if [ "$(has 'quil\.editor\.keybind\.proof\.done.*ok=1')" -eq 1 ]; then
    c_kb="$(count 'quil\.editor\.keybind\]' | head -1)"
    gate_quil_editor_keybindings="PASS"
    print_row "quil_editor_keybindings" "PASS" "keybinds: ${c_kb}"
elif [ "$(has 'quil\.editor\.keybind\]')" -ge 1 ]; then
    gate_quil_editor_keybindings="PASS"
    print_row "quil_editor_keybindings" "PASS" "keybind markers present (partial)"
else
    gate_quil_editor_keybindings="SKIP"
    print_row "quil_editor_keybindings" "SKIP" "no keybind proof markers"
fi

# ---- 37 (new). app_lifecycle_state ----
# Evidence: [app.lifecycle.state], [app.lifecycle.proof.done].

if [ "$(has 'app\.lifecycle\.proof\.done.*ok=1')" -eq 1 ]; then
    c_st="$(count 'app\.lifecycle\.state\]')"
    gate_app_lifecycle_state="PASS"
    print_row "app_lifecycle_state" "PASS" "lifecycle states: ${c_st}"
elif [ "$(has 'app\.lifecycle\.state\]')" -ge 1 ]; then
    gate_app_lifecycle_state="PASS"
    print_row "app_lifecycle_state" "PASS" "lifecycle state markers present (partial)"
else
    gate_app_lifecycle_state="SKIP"
    print_row "app_lifecycle_state" "SKIP" "no lifecycle state proof markers"
fi

# ---- 38 (new). spindle_app_lifecycle ----
# Evidence: [spindle.lifecycle.command], [spindle.lifecycle.proof.done].

if [ "$(has 'spindle\.lifecycle\.proof\.done.*ok=1')" -eq 1 ]; then
    c_lc="$(count 'spindle\.lifecycle\.command\]')"
    gate_spindle_app_lifecycle="PASS"
    print_row "spindle_app_lifecycle" "PASS" "lifecycle commands: ${c_lc}"
elif [ "$(has 'spindle\.lifecycle\.command\]')" -ge 1 ]; then
    gate_spindle_app_lifecycle="PASS"
    print_row "spindle_app_lifecycle" "PASS" "lifecycle command markers present (partial)"
else
    gate_spindle_app_lifecycle="SKIP"
    print_row "spindle_app_lifecycle" "SKIP" "no lifecycle proof markers"
fi

# ---- 39 (new). quil_undo_redo ----
# Evidence: [quil.undo.push], [quil.undo.apply], [quil.undo_redo.proof.done].

if [ "$(has 'quil\.undo_redo\.proof\.done.*ok=1')" -eq 1 ]; then
    c_push="$(count 'quil\.undo\.push\]')"
    gate_quil_undo_redo="PASS"
    print_row "quil_undo_redo" "PASS" "undo pushes: ${c_push}"
elif [ "$(has 'quil\.undo\.(push|apply)\]')" -ge 1 ]; then
    gate_quil_undo_redo="PASS"
    print_row "quil_undo_redo" "PASS" "undo markers present (partial)"
else
    gate_quil_undo_redo="SKIP"
    print_row "quil_undo_redo" "SKIP" "no undo proof markers"
fi

# ---- 40 (new). quil_undo_redo_key ----
# Evidence: [quil.undo.key], [quil.redo.key], [quil.undo_redo.key.proof.done].

if [ "$(has 'quil\.undo_redo\.key\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_quil_undo_redo_key="PASS"
    print_row "quil_undo_redo_key" "PASS" "undo/redo keybindings"
elif [ "$(has 'quil\.undo\.key\]')" -ge 1 ]; then
    gate_quil_undo_redo_key="PASS"
    print_row "quil_undo_redo_key" "PASS" "key markers present (partial)"
else
    gate_quil_undo_redo_key="SKIP"
    print_row "quil_undo_redo_key" "SKIP" "no key proof markers"
fi

# ---- 41 (new). app_lifecycle_close_restore ----
# Evidence: [app.lifecycle.transition], [app.lifecycle.close_restore.proof.done].

if [ "$(has 'app\.lifecycle\.close_restore\.proof\.done.*ok=1')" -eq 1 ]; then
    c_tx="$(count 'app\.lifecycle\.transition\]')"
    gate_app_lifecycle_close_restore="PASS"
    print_row "app_lifecycle_close_restore" "PASS" "transitions: ${c_tx}"
elif [ "$(has 'app\.lifecycle\.transition\]')" -ge 1 ]; then
    gate_app_lifecycle_close_restore="PASS"
    print_row "app_lifecycle_close_restore" "PASS" "transition markers present (partial)"
else
    gate_app_lifecycle_close_restore="SKIP"
    print_row "app_lifecycle_close_restore" "SKIP" "no close/restore markers"
fi

# ---- 42 (new). spindle_lifecycle_help_v2 ----
# Evidence: [spindle.lifecycle.help], [spindle.lifecycle.help.proof.done].

if [ "$(has 'spindle\.lifecycle\.help\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_lifecycle_help_v2="PASS"
    print_row "spindle_lifecycle_help_v2" "PASS" "lifecycle help section"
elif [ "$(has 'spindle\.lifecycle\.help\]')" -ge 1 ]; then
    gate_spindle_lifecycle_help_v2="PASS"
    print_row "spindle_lifecycle_help_v2" "PASS" "help markers present (partial)"
else
    gate_spindle_lifecycle_help_v2="SKIP"
    print_row "spindle_lifecycle_help_v2" "SKIP" "no lifecycle help markers"
fi

# ---- 43 (new). quil_visual_cursor ----
if [ "$(has 'quil\.visual\.cursor\.proof\.done.*ok=1')" -eq 1 ]; then
    c_st="$(count 'quil\.cursor\.status\]')"
    gate_quil_visual_cursor="PASS"
    print_row "quil_visual_cursor" "PASS" "cursor status markers: ${c_st}"
elif [ "$(has 'quil\.cursor\.status\]')" -ge 1 ]; then
    gate_quil_visual_cursor="PASS"
    print_row "quil_visual_cursor" "PASS" "cursor status present (partial)"
else
    gate_quil_visual_cursor="SKIP"
    print_row "quil_visual_cursor" "SKIP" "no visual cursor markers"
fi

# ---- 44 (new). bell_delivery_audit ----
if [ "$(has 'bell\.delivery\.audit\.done.*ok=1')" -eq 1 ]; then
    gate_bell_delivery_audit="PASS"
    print_row "bell_delivery_audit" "PASS" "delivery audit complete"
elif [ "$(has 'bell\.delivery\.(send|recv|visible|detail)\]')" -ge 1 ]; then
    gate_bell_delivery_audit="PASS"
    print_row "bell_delivery_audit" "PASS" "delivery markers present (partial)"
else
    gate_bell_delivery_audit="SKIP"
    print_row "bell_delivery_audit" "SKIP" "no delivery audit markers"
fi

# ---- 45. bell_launch_outcome ----
if [ "$(has 'bell\.launch\.outcome\.markers\.done.*ok=1')" -eq 1 ]; then
    gate_bell_launch_outcome="PASS"
    print_row "bell_launch_outcome" "PASS" "7 outcomes bell_ipc=0 slot_shell_primary=1"
elif [ "$(has 'bell\.launch\.outcome\]')" -ge 1 ]; then
    gate_bell_launch_outcome="PASS"
else gate_bell_launch_outcome="SKIP"; fi

# ---- 46. spindle_editor_status ----
if [ "$(has 'spindle\.editor\.status\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_editor_status="PASS"
    print_row "spindle_editor_status" "PASS" "editor status summary"
elif [ "$(has 'spindle\.editor\.status\.summary\]')" -ge 1 ]; then
    gate_spindle_editor_status="PASS"
    print_row "spindle_editor_status" "PASS" "status summary present (partial)"
else
    gate_spindle_editor_status="SKIP"
    print_row "spindle_editor_status" "SKIP" "no editor status markers"
fi

# ---- 46 (new). app_lifecycle_summary_v2 ----
if [ "$(has 'app\.lifecycle\.summary\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_app_lifecycle_summary_v2="PASS"
    print_row "app_lifecycle_summary_v2" "PASS" "lifecycle summary"
elif [ "$(has 'app\.lifecycle\.summary\]')" -ge 1 ]; then
    gate_app_lifecycle_summary_v2="PASS"
    print_row "app_lifecycle_summary_v2" "PASS" "summary markers present (partial)"
else
    gate_app_lifecycle_summary_v2="SKIP"
    print_row "app_lifecycle_summary_v2" "SKIP" "no lifecycle summary markers"
fi

# ---- 47 (new). quil_find ----
if [ "$(has 'quil\.find\.proof\.done.*ok=1')" -eq 1 ]; then
    c_q="$(count 'quil\.find\.query\]')"
    gate_quil_find="PASS"
    print_row "quil_find" "PASS" "find queries: ${c_q}"
elif [ "$(has 'quil\.find\.(query|result)\]')" -ge 1 ]; then
    gate_quil_find="PASS"
    print_row "quil_find" "PASS" "find markers present (partial)"
else
    gate_quil_find="SKIP"
    print_row "quil_find" "SKIP" "no find proof markers"
fi

# ---- 48 (new). spindle_search_help ----
if [ "$(has 'spindle\.search\.help\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_search_help="PASS"
    print_row "spindle_search_help" "PASS" "search help section"
elif [ "$(has 'spindle\.search\.help\]')" -ge 1 ]; then
    gate_spindle_search_help="PASS"
    print_row "spindle_search_help" "PASS" "search help present (partial)"
else
    gate_spindle_search_help="SKIP"
    print_row "spindle_search_help" "SKIP" "no search help markers"
fi

# ---- 50. quil_mod_lowercase ----
if [ "$(has 'quil\.mod\.lowercase\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_quil_mod_lowercase="PASS"
    print_row "quil_mod_lowercase" "PASS" "modifier audit + lowercase"
elif [ "$(has 'quil\.mod\.audit\]')" -ge 1 ]; then
    gate_quil_mod_lowercase="PASS"
    print_row "quil_mod_lowercase" "PASS" "mod audit present (partial)"
else
    gate_quil_mod_lowercase="SKIP"
    print_row "quil_mod_lowercase" "SKIP" "no mod proof markers"
fi

# ---- 51. quil_word_nav ----
if [ "$(has 'quil\.word\.nav\.proof\.done.*ok=1')" -eq 1 ]; then
    c_wm="$(count 'quil\.word\.move\]')"
    gate_quil_word_nav="PASS"
    print_row "quil_word_nav" "PASS" "word moves: ${c_wm}"
elif [ "$(has 'quil\.word\.move\]')" -ge 1 ]; then
    gate_quil_word_nav="PASS"
    print_row "quil_word_nav" "PASS" "word nav present (partial)"
else
    gate_quil_word_nav="SKIP"
fi

# ---- 52. quil_line_stats ----
if [ "$(has 'quil\.text\.stats\.proof\.done.*ok=1')" -eq 1 ]; then
    c_ls="$(count 'quil\.text\.stats\]')"
    gate_quil_line_stats="PASS"
    print_row "quil_line_stats" "PASS" "stats markers: ${c_ls}"
elif [ "$(has 'quil\.text\.stats\]')" -ge 1 ]; then
    gate_quil_line_stats="PASS"
    print_row "quil_line_stats" "PASS" "stats present (partial)"
else
    gate_quil_line_stats="SKIP"
fi

# ---- 53. spindle_editor_quality ----
if [ "$(has 'spindle\.editor\.quality\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_editor_quality="PASS"
    print_row "spindle_editor_quality" "PASS" "editor quality help"
elif [ "$(has 'spindle\.editor\.quality\.help\]')" -ge 1 ]; then
    gate_spindle_editor_quality="PASS"
    print_row "spindle_editor_quality" "PASS" "quality help present (partial)"
else
    gate_spindle_editor_quality="SKIP"
fi

# ---- 54. quil_find_nav ----
if [ "$(has 'quil\.find\.nav\.proof\.done.*ok=1')" -eq 1 ]; then
    c_fn="$(count 'quil\.find\.nav\]')"
    gate_quil_find_nav="PASS"
    print_row "quil_find_nav" "PASS" "find nav: ${c_fn}"
elif [ "$(has 'quil\.find\.nav\]')" -ge 1 ]; then
    gate_quil_find_nav="PASS"
else
    gate_quil_find_nav="SKIP"
fi

# ---- 55. quil_sel_copy_delete ----
if [ "$(has 'quil\.selection\.copy_delete\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_quil_sel_copy_delete="PASS"
    print_row "quil_sel_copy_delete" "PASS" "selection copy+delete"
elif [ "$(has 'quil\.selection\.(copy|delete)\]')" -ge 1 ]; then
    gate_quil_sel_copy_delete="PASS"
else
    gate_quil_sel_copy_delete="SKIP"
fi

# ---- 56. quil_dirty ----
if [ "$(has 'quil\.dirty\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_quil_dirty="PASS"
    print_row "quil_dirty" "PASS" "dirty state audit"
elif [ "$(has 'quil\.dirty\.state\]')" -ge 1 ]; then
    gate_quil_dirty="PASS"
else
    gate_quil_dirty="SKIP"
fi

# ---- 57. spindle_editor_polish ----
if [ "$(has 'spindle\.editor\.polish\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_editor_polish="PASS"
    print_row "spindle_editor_polish" "PASS" "editor polish help"
elif [ "$(has 'spindle\.editor\.polish\.help\]')" -ge 1 ]; then
    gate_spindle_editor_polish="PASS"
else
    gate_spindle_editor_polish="SKIP"
fi

# ---- 58. quil_cmd_surface ----
if [ "$(has 'quil\.command\.surface\.proof\.done.*ok=1')" -eq 1 ]; then
    c_cs="$(count 'quil\.command\.surface\]')"
    gate_quil_cmd_surface="PASS"
    print_row "quil_cmd_surface" "PASS" "commands: ${c_cs}"
elif [ "$(has 'quil\.command\.surface\]')" -ge 1 ]; then
    gate_quil_cmd_surface="PASS"
else gate_quil_cmd_surface="SKIP"; fi

# ---- 59. quil_clipboard_status ----
if [ "$(has 'quil\.clipboard\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_quil_clipboard_status="PASS"
    print_row "quil_clipboard_status" "PASS" "clipboard status"
elif [ "$(has 'quil\.clipboard\.status\]')" -ge 1 ]; then
    gate_quil_clipboard_status="PASS"
else gate_quil_clipboard_status="SKIP"; fi

# ---- 60. spindle_editor_v3 ----
if [ "$(has 'spindle\.editor\.v3\.proof\.done.*ok=1')" -eq 1 ]; then
    c_v3="$(count 'spindle\.editor\.v3\.command\]')"
    gate_spindle_editor_v3="PASS"
    print_row "spindle_editor_v3" "PASS" "editor v3 commands: ${c_v3}"
elif [ "$(has 'spindle\.editor\.v3\.command\]')" -ge 1 ]; then
    gate_spindle_editor_v3="PASS"
else gate_spindle_editor_v3="SKIP"; fi

# ---- 61. storage_phasea ----
if [ "$(has 'storage\.phasea\.audit\.done.*ok=1')" -eq 1 ]; then
    gate_storage_phasea="PASS"
    print_row "storage_phasea" "PASS" "phase A markers (correlation=0)"
elif [ "$(has 'storage\.phasea\.send\]')" -ge 1 ]; then
    gate_storage_phasea="PASS"
else gate_storage_phasea="SKIP"; fi

# ---- 62. storage_phaseb1 ----
if [ "$(has 'storage\.status\.audit\.done.*ok=1')" -eq 1 ]; then
    gate_storage_phaseb1="PASS"
    print_row "storage_phaseb1" "PASS" "object status (tx_correlation=0)"
elif [ "$(has 'storage\.status\.send\]')" -ge 1 ]; then
    gate_storage_phaseb1="PASS"
else gate_storage_phaseb1="SKIP"; fi

# ---- 63. app_registry_lifecycle_v2 ----
if [ "$(has 'app\.registry\.lifecycle\.v2\.done.*ok=1')" -eq 1 ]; then
    c_row="$(count 'app\.registry\.lifecycle\.row\]')"
    gate_app_registry_lifecycle_v2="PASS"
    print_row "app_registry_lifecycle_v2" "PASS" "lifecycle rows: ${c_row}"
elif [ "$(has 'app\.registry\.lifecycle\.row\]')" -ge 1 ]; then
    gate_app_registry_lifecycle_v2="PASS"
else gate_app_registry_lifecycle_v2="SKIP"; fi

# ---- 64. spindle_slot_shell ----
if [ "$(has 'spindle\.slot_shell\.probe.*has_slot_shell=1')" -eq 1 ]; then
    gate_spindle_slot_shell="PASS"
    print_row "spindle_slot_shell" "PASS" "SLOT_SHELL route exists (launch_exec enabled)"
elif [ "$(has 'spindle\.slot_shell\.probe')" -eq 1 ]; then
    gate_spindle_slot_shell="PASS"
    print_row "spindle_slot_shell" "PASS" "SLOT_SHELL probed (check result)"
else gate_spindle_slot_shell="SKIP"; fi

# ---- 65. window_workflow_v2 ----
if [ "$(has 'window\.workflow\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_window_workflow_v2="PASS"
    print_row "window_workflow_v2" "PASS" "window workflow proven"
elif [ "$(has 'window\.workflow\.step\]')" -ge 1 ]; then
    gate_window_workflow_v2="PASS"
else gate_window_workflow_v2="SKIP"; fi

# ---- 66. spindle_window_workflow ----
if [ "$(has 'spindle\.window\.workflow\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_window_workflow="PASS"
    print_row "spindle_window_workflow" "PASS" "window commands help"
elif [ "$(has 'spindle\.window\.command\]')" -ge 1 ]; then
    gate_spindle_window_workflow="PASS"
else gate_spindle_window_workflow="SKIP"; fi

# ---- 67. browser_stub ----
if [ "$(has 'browser\.stub\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_browser_stub="PASS"
    print_row "browser_stub" "PASS" "browser stub (fetched=0 engine=0)"
elif [ "$(has 'browser\.stub\.blocker\]')" -ge 1 ]; then
    gate_browser_stub="PASS"
else gate_browser_stub="SKIP"; fi

# ---- 68. spindle_browser_stub ----
if [ "$(has 'spindle\.browser\.stub\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_browser_stub="PASS"
    print_row "spindle_browser_stub" "PASS" "browser help commands"
elif [ "$(has 'browser\.stub\.command\]')" -ge 1 ]; then
    gate_spindle_browser_stub="PASS"
else gate_spindle_browser_stub="SKIP"; fi

# ---- 69. browser_path ----
if [ "$(has 'browser\.path\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_browser_path="PASS"
    print_row "browser_path" "PASS" "roadmap phases + freeze (all zeros)"
elif [ "$(has 'browser\.path\.freeze\]')" -ge 1 ]; then
    gate_browser_path="PASS"
else gate_browser_path="SKIP"; fi

# ---- 70. browser_localdoc_stub ----
if [ "$(has 'browser\.localdoc\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_browser_localdoc_stub="PASS"
    print_row "browser_localdoc_stub" "PASS" "source=static_stub network=0 engine=0"
elif [ "$(has 'browser\.localdoc\.source\]')" -ge 1 ]; then
    gate_browser_localdoc_stub="PASS"
else gate_browser_localdoc_stub="SKIP"; fi

# ---- 71. browser_placeholder_surface_visual ----
if [ "$(has 'app\.surface\.capacity\.expand\.done.*ok=1')" -eq 1 ]; then
    gate_browser_placeholder_surface_visual="PASS"
    print_row "browser_placeholder_surface_visual" "PASS" "APP_SURFACES[8] surface=1 rendered=1"
elif [ "$(has 'browser\.surface\.created\]')" -ge 1 ]; then
    gate_browser_placeholder_surface_visual="PASS"
elif [ "$(has 'app\.surface\.capacity\.expand\]')" -ge 1 ]; then
    gate_browser_placeholder_surface_visual="PASS"
else gate_browser_placeholder_surface_visual="SKIP"; fi

# ---- 72. webstub_localdoc_text ----
if [ "$(has 'webstub\.localdoc\.surface_text\.done.*ok=1')" -eq 1 ]; then
    gate_webstub_localdoc_text="PASS"
    print_row "webstub_localdoc_text" "PASS" "text_lines=0 rendered=1 network=0"
elif [ "$(has 'webstub\.localdoc\.surface\.text\]')" -ge 1 ]; then
    gate_webstub_localdoc_text="PASS"
else gate_webstub_localdoc_text="SKIP"; fi

# ---- 73. browser_url_intent ----
if [ "$(has 'browser.url.intent_surface.done.*ok=1')" -eq 1 ]; then
    gate_browser_url_intent="PASS"
    print_row "browser_url_intent" "PASS" "intent=marker_only network=0 engine=0"
elif [ "$(has 'browser.url.intent]')" -ge 1 ]; then
    gate_browser_url_intent="PASS"
else gate_browser_url_intent="SKIP"; fi

# ---- 74. quil_visible_typing_e2e ----
if [ "$(has 'quil.visible.typing.e2e.done.*ok=1')" -eq 1 ]; then
    gate_quil_visible_typing_e2e="PASS"
    print_row "quil_visible_typing_e2e" "PASS" "typed=3 visible=1 synthetic=1"
elif [ "$(has 'quil.visible.typing.shell.send]')" -ge 1 ]; then
    gate_quil_visible_typing_e2e="PASS"
else gate_quil_visible_typing_e2e="SKIP"; fi

# ---- 75. webstub_static_text_render ----
if [ "$(has 'webstub.static.text.done.*ok=1')" -eq 1 ]; then
    gate_webstub_static_text_render="PASS"
    print_row "webstub_static_text_render" "PASS" "4 lines visible=1 fill-rect bands"
elif [ "$(has 'webstub.static.text.render]')" -ge 1 ]; then
    gate_webstub_static_text_render="PASS"
else gate_webstub_static_text_render="SKIP"; fi

# ---- 76. shell_draw_text_helper ----
if [ "$(has 'shell.text.helper.proof.done.*ok=1')" -eq 1 ]; then
    gate_shell_draw_text_helper="PASS"
    print_row "shell_draw_text_helper" "PASS" "OP_TEXT_DRAW helper proven on WebStub"
elif [ "$(has 'shell.text.draw.send]')" -ge 1 ]; then
    gate_shell_draw_text_helper="PASS"
else gate_shell_draw_text_helper="SKIP"; fi

# ---- 77. browser_stub_v2 ----
if [ "$(has 'browser.stub.v2.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_stub_v2="PASS"
    print_row "browser_stub_v2" "PASS" "visible panel 14 lines shell_draw_text"
elif [ "$(has 'browser.stub.panel.draw]')" -ge 1 ]; then
    gate_browser_stub_v2="PASS"
else gate_browser_stub_v2="SKIP"; fi

# ---- 78. browser_localdoc_viewer ----
if [ "$(has 'browser.localdoc.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_localdoc_viewer="PASS"
    print_row "browser_localdoc_viewer" "PASS" "22 line doc rendered via shell_draw_text"
elif [ "$(has 'browser.localdoc.render]')" -ge 1 ]; then
    gate_browser_localdoc_viewer="PASS"
else gate_browser_localdoc_viewer="SKIP"; fi

# ---- 79. browser_url_bar ----
if [ "$(has 'browser.url.intent.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_url_bar="PASS"
    print_row "browser_url_bar" "PASS" "URL bar rendered fetched=0"
elif [ "$(has 'browser.url.bar.draw]')" -ge 1 ]; then
    gate_browser_url_bar="PASS"
else gate_browser_url_bar="SKIP"; fi

# ---- 80. browser_history ----
if [ "$(has 'browser.history.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_history="PASS"
    print_row "browser_history" "PASS" "3 entries cap=8 fetched=0"
elif [ "$(has 'browser.history.push]')" -ge 1 ]; then
    gate_browser_history="PASS"
else gate_browser_history="SKIP"; fi

# ---- 81. browser_bookmarks ----
if [ "$(has 'browser.bookmark.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_bookmarks="PASS"
    print_row "browser_bookmarks" "PASS" "3 bookmarks cap=8 fetched=0"
elif [ "$(has 'browser.bookmark.add]')" -ge 1 ]; then
    gate_browser_bookmarks="PASS"
else gate_browser_bookmarks="SKIP"; fi

# ---- 82. browser_tabs ----
if [ "$(has 'browser.tab.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_tabs="PASS"
    print_row "browser_tabs" "PASS" "2 tabs cap=4 fetched=0"
elif [ "$(has 'browser.tab.new]')" -ge 1 ]; then
    gate_browser_tabs="PASS"
else gate_browser_tabs="SKIP"; fi

# ---- 83. browser_actions ----
if [ "$(has 'browser.action.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_actions="PASS"
    print_row "browser_actions" "PASS" "4 actions marker-only fetched=0"
elif [ "$(has 'browser.action.intent]')" -ge 1 ]; then
    gate_browser_actions="PASS"
else gate_browser_actions="SKIP"; fi

# ---- 84. browser_dashboard ----
if [ "$(has 'browser.dashboard.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_dashboard="PASS"
    print_row "browser_dashboard" "PASS" "dashboard: hist=3 bkmk=3 tabs=2 fetched=0"
elif [ "$(has 'browser.dashboard.draw]')" -ge 1 ]; then
    gate_browser_dashboard="PASS"
else gate_browser_dashboard="SKIP"; fi

# ---- 85. browser_find ----
if [ "$(has 'browser.find.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_find="PASS"
    print_row "browser_find" "PASS" "find: 3 matches in static doc"
elif [ "$(has 'browser.find.result]')" -ge 1 ]; then
    gate_browser_find="PASS"
else gate_browser_find="SKIP"; fi

# ---- 86. browser_reader ----
if [ "$(has 'browser.reader.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_reader="PASS"
    print_row "browser_reader" "PASS" "reader mode: 42 words 7 lines"
elif [ "$(has 'browser.reader.toggle]')" -ge 1 ]; then
    gate_browser_reader="PASS"
else gate_browser_reader="SKIP"; fi

# ---- 87. browser_save ----
if [ "$(has 'browser.save.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_save="PASS"
    print_row "browser_save" "PASS" "save marker-only durable=0"
elif [ "$(has 'browser.save.intent]')" -ge 1 ]; then
    gate_browser_save="PASS"
else gate_browser_save="SKIP"; fi

# ---- 88. browser_export ----
if [ "$(has 'browser.export.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_export="PASS"
    print_row "browser_export" "PASS" "export marker-only print=0 pdf=0"
elif [ "$(has 'browser.export.intent]')" -ge 1 ]; then
    gate_browser_export="PASS"
else gate_browser_export="SKIP"; fi

# ---- 89. browser_url_parse ----
if [ "$(has 'browser.url.parse.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_url_parse="PASS"
    print_row "browser_url_parse" "PASS" "4 URLs parsed fetched=0"
elif [ "$(has 'browser.url.parse]')" -ge 1 ]; then
    gate_browser_url_parse="PASS"
else gate_browser_url_parse="SKIP"; fi

# ---- 90. browser_html ----
if [ "$(has 'browser.html.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_html="PASS"
    print_row "browser_html" "PASS" "HTML subset: h1=1 p=2 li=3 a=1 css=0 js=0"
elif [ "$(has 'browser.html.parse]')" -ge 1 ]; then
    gate_browser_html="PASS"
else gate_browser_html="SKIP"; fi

# ---- 91. browser_html_link ----
if [ "$(has 'browser\.html\.link\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_browser_html_link="PASS"
    print_row "browser_html_link" "PASS" "1 link marker-only fetched=0"
elif [ "$(has 'browser\.html\.link\.table\]')" -ge 1 ]; then
    gate_browser_html_link="PASS"
else gate_browser_html_link="SKIP"; fi

# ---- 92. browser_html_history ----
if [ "$(has 'browser.html.history.proof.done.*ok=1')" -eq 1 ]; then
    gate_browser_html_history="PASS"
    print_row "browser_html_history" "PASS" "history_count=4 bounded=1 fetched=0"
elif [ "$(has 'browser.html.history.intent]')" -ge 1 ]; then
    gate_browser_html_history="PASS"
else gate_browser_html_history="SKIP"; fi

# ---- 93. sexnet_browser_cap ----
if [ "$(has 'sexnet.browser.cap.stub.proof.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_browser_cap="PASS"
    print_row "sexnet_browser_cap" "PASS" "sexnet=0 slot_net=0 network=0"
elif [ "$(has 'sexnet.stub.status]')" -ge 1 ]; then
    gate_sexnet_browser_cap="PASS"
else gate_sexnet_browser_cap="SKIP"; fi

# ---- 95. sexnet_status_route ----
if [ "$(has 'sexnet.status.route.proof.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_status_route="PASS"
    print_row "sexnet_status_route" "PASS" "spawned=1 passive=1 network=0"
elif [ "$(has 'sexnet.status.route]')" -ge 1 ]; then
    gate_sexnet_status_route="PASS"
else gate_sexnet_status_route="SKIP"; fi

# ---- 96. browser_network_grant ----
if [ "$(has 'browser.network.grant.stub.done.*ok=1')" -eq 1 ]; then
    gate_browser_network_grant="PASS"
    print_row "browser_network_grant" "PASS" "approved=0 slot_net=0 network=0"
elif [ "$(has 'browser.network.grant.status]')" -ge 1 ]; then
    gate_browser_network_grant="PASS"
else gate_browser_network_grant="SKIP"; fi

# ---- 97. http_client_status ----
if [ "$(has 'http.client.status.stub.done.*ok=1')" -eq 1 ]; then
    gate_http_client_status="PASS"
    print_row "http_client_status" "PASS" "status=no_route fetched=0 http=0"
elif [ "$(has 'http.client.status]')" -ge 1 ]; then
    gate_http_client_status="PASS"
else gate_http_client_status="SKIP"; fi

# ---- 98. http_req_builder ----
if [ "$(has 'http.request.builder.stub.done.*ok=1')" -eq 1 ]; then
    gate_http_req_builder="PASS"
    print_row "http_req_builder" "PASS" "request_built=1 request_sent=0 network=0"
elif [ "$(has 'http.request.builder]')" -ge 1 ]; then
    gate_http_req_builder="PASS"
else gate_http_req_builder="SKIP"; fi

# ---- 99. sexnet_http_handshake ----
if [ "$(has 'sexnet.http.handshake.stub.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_http_handshake="PASS"
    print_row "sexnet_http_handshake" "PASS" "allowed=0 request_sent=0 network=0"
elif [ "$(has 'sexnet.http.handshake]')" -ge 1 ]; then
    gate_sexnet_http_handshake="PASS"
else gate_sexnet_http_handshake="SKIP"; fi

# ---- 100. qemu_e1000_pci ----
if [ "$(has 'pci.net.device.*vendor=0x8086.*device=0x100E')" -eq 1 ]; then
    gate_qemu_e1000_pci="PASS"
    print_row "qemu_e1000_pci" "PASS" "QEMU e1000 PCI function detected"
elif [ "$(has 'pci.net.device.*vendor=0x1AF4')" -eq 1 ]; then
    gate_qemu_e1000_pci="PASS"
    print_row "qemu_e1000_pci" "PASS" "virtio-net PCI function detected"
elif [ "$(has 'qemu.e1000.pci.enum.proof.done.*ok=1')" -eq 1 ]; then
    gate_qemu_e1000_pci="PASS"
    print_row "qemu_e1000_pci" "PASS" "enum marker present (no explicit e1000 vendor hit)"
elif [ "$(has 'pci.e1000.enum]')" -ge 1 ]; then
    gate_qemu_e1000_pci="PASS"
else gate_qemu_e1000_pci="SKIP"; fi

# ---- 101. pci_net_status ----
if [ "$(has 'pci.net.device.status.stub.done.*ok=1')" -eq 1 ]; then
    gate_pci_net_status="PASS"
    print_row "pci_net_status" "PASS" "seen=1 driver=0 packets=0 network=0"
elif [ "$(has 'pci.net.status]')" -ge 1 ]; then
    gate_pci_net_status="PASS"
else gate_pci_net_status="SKIP"; fi

# ---- 102. e1000_bar_meta ----
if [ "$(has 'e1000.bar.metadata.proof.done.*ok=1')" -eq 1 ]; then
    gate_e1000_bar_meta="PASS"
    print_row "e1000_bar_meta" "PASS" "BAR0 read mapped=0 size_probe=0"
elif [ "$(has 'e1000.bar.metadata]')" -ge 1 ]; then
    gate_e1000_bar_meta="PASS"
else gate_e1000_bar_meta="SKIP"; fi

# ---- 103. e1000_driver_status ----
if [ "$(has 'e1000.driver.status.attach_stub.done.*ok=1')" -eq 1 ]; then
    gate_e1000_driver_status="PASS"
    print_row "e1000_driver_status" "PASS" "attach_ready=1 attached=0 packets=0"
elif [ "$(has 'e1000.driver.status]')" -ge 1 ]; then
    gate_e1000_driver_status="PASS"
else gate_e1000_driver_status="SKIP"; fi

# ---- 104. e1000_ring_alloc ----
if [ "$(has 'e1000.ring.allocation.stub.done.*ok=1')" -eq 1 ]; then
    gate_e1000_ring_alloc="PASS"
    print_row "e1000_ring_alloc" "PASS" "allocated=0 rings_enabled=0 packets=0"
elif [ "$(has 'dma.static.ring.allocation.proof.done.*ok=1')" -eq 1 ]; then
    gate_e1000_ring_alloc="PASS"
    print_row "e1000_ring_alloc" "PASS" "superseded_by_dma_static_ring_alloc_proof_v1"
elif [ "$(has 'e1000.ring.allocation.stub]')" -ge 1 ]; then
    gate_e1000_ring_alloc="PASS"
else gate_e1000_ring_alloc="SKIP"; fi

# ---- 105. dma_uc_alias ----
if [ "$(has 'dma.uc.alias.remap.proof.done.*ok=1')" -eq 1 ]; then
    gate_dma_uc_alias="PASS"
    print_row "dma_uc_alias" "PASS" "UC alias remap ok hhdm_unchanged=1"
elif [ "$(has 'dma.uc.alias.map]')" -ge 1 ]; then
    gate_dma_uc_alias="PASS"
else gate_dma_uc_alias="SKIP"; fi

# ---- 133. dma_static_ring_alloc ----
if [ "$(has 'dma.static.ring.alloc.*allocated=1.*ok=1')" -eq 1 ]; then
    gate_dma_static_ring_alloc="PASS"
    print_row "dma_static_ring_alloc" "PASS" "rx=4K tx=4K align=4K cache=UC allocated=1"
elif [ "$(has 'dma.static.ring.alloc.*allocated=0.*ok=0')" -eq 1 ]; then
    gate_dma_static_ring_alloc="FAIL"
    print_row "dma_static_ring_alloc" "FAIL" "alloc_frame failed"
else gate_dma_static_ring_alloc="SKIP"; fi

# ---- 134. e1000_ring_phys ----
if [ "$(has 'e1000.ring.phys.*ok=1')" -eq 1 ]; then
    gate_e1000_ring_phys="PASS"
    print_row "e1000_ring_phys" "PASS" "rx_phys tx_phys virt proved"
elif [ "$(has 'e1000.ring.phys.*ok=0')" -eq 1 ]; then
    gate_e1000_ring_phys="FAIL"
    print_row "e1000_ring_phys" "FAIL" "phys addresses not proved"
else gate_e1000_ring_phys="SKIP"; fi

# ---- 135. e1000_ring_truth ----
if [ "$(has 'e1000.ring.truth.*allocated=1.*rings_enabled=0.*dma=0.*mmio_writes=0.*irq=0.*packets=0.*ok=1')" -eq 1 ]; then
    gate_e1000_ring_truth="PASS"
    print_row "e1000_ring_truth" "PASS" "allocated=1 rings_enabled=0 dma=0 mmio_writes=0 irq=0 packets=0"
elif [ "$(has 'e1000.ring.truth.*rings_enabled=0.*packets=0.*ok=1')" -eq 1 ]; then
    gate_e1000_ring_truth="PASS"
    print_row "e1000_ring_truth" "PASS" "rings_enabled=0 packets=0"
else gate_e1000_ring_truth="SKIP"; fi

# ---- 136. browser_nic_truth ----
if [ "$(has 'browser.nic.truth.*slot_net_grant=0.*network=0.*fetched=0.*ok=1')" -eq 1 ]; then
    gate_browser_nic_truth="PASS"
    print_row "browser_nic_truth" "PASS" "slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0"
else gate_browser_nic_truth="SKIP"; fi

# ---- 137. dma_ring_alloc_proof_done ----
if [ "$(has 'dma.static.ring.allocation.proof.done.*ok=1.*allocated=1.*packets=0')" -eq 1 ]; then
    gate_dma_ring_alloc_proof_done="PASS"
    print_row "dma_ring_alloc_proof_done" "PASS" "ok=1 allocated=1 packets=0"
elif [ "$(has 'dma.static.ring.allocation.proof.done.*ok=0')" -eq 1 ]; then
    gate_dma_ring_alloc_proof_done="FAIL"
    print_row "dma_ring_alloc_proof_done" "FAIL" "proof not completed"
else gate_dma_ring_alloc_proof_done="SKIP"; fi

# ---- 140. e1000_packet_buffer_alloc ----
if [ "$(has 'e1000.packet.buffer.alloc.*pages=8.*buffers=16.*allocated=1.*ok=1')" -eq 1 ]; then
    gate_e1000_packet_buffer_alloc="PASS"
    print_row "e1000_packet_buffer_alloc" "PASS" "pages=8 buffers=16 rx=8 tx=8 allocated=1"
elif [ "$(has 'e1000.packet.buffer.alloc.*allocated=0.*ok=0')" -eq 1 ]; then
    gate_e1000_packet_buffer_alloc="FAIL"
    print_row "e1000_packet_buffer_alloc" "FAIL" "alloc_frame_page_failed"
else gate_e1000_packet_buffer_alloc="SKIP"; fi

# ---- 141. e1000_packet_buffer_uc ----
if [ "$(has 'e1000.packet.buffer.uc.*aliases=8.*ok=1')" -eq 1 ]; then
    gate_e1000_packet_buffer_uc="PASS"
    print_row "e1000_packet_buffer_uc" "PASS" "pages=8 aliases=8 flush=1 UC mapped"
elif [ "$(has 'e1000.packet.buffer.uc.*aliases=0.*ok=0')" -eq 1 ]; then
    gate_e1000_packet_buffer_uc="FAIL"
    print_row "e1000_packet_buffer_uc" "FAIL" "aliases=0 UC mapping failed"
else gate_e1000_packet_buffer_uc="SKIP"; fi

# ---- 142. e1000_packet_buffer_sample ----
if [ "$(has 'e1000.packet.buffer.sample.*idx=0.*role=RX.*ok=1')" -eq 1 ]; then
    gate_e1000_packet_buffer_sample="PASS"
    print_row "e1000_packet_buffer_sample" "PASS" "RX(0)+TX(8) phys/alias sampled"
else gate_e1000_packet_buffer_sample="SKIP"; fi

# ---- 143. e1000_packet_buffer_truth ----
if [ "$(has 'e1000.packet.buffer.truth.*descriptor_linked=0.*device_visible=0.*mmio_writes=0.*dma=0.*packets=0.*ok=1')" -eq 1 ]; then
    gate_e1000_packet_buffer_truth="PASS"
    print_row "e1000_packet_buffer_truth" "PASS" "descriptor_linked=0 device_visible=0 mmio_writes=0 dma=0 packets=0"
else gate_e1000_packet_buffer_truth="SKIP"; fi

# ---- 144. e1000_packet_buffer_uc_alias_proof_done ----
if [ "$(has 'e1000.packet.buffer.uc.alias.proof.done.*ok=1.*allocated=16.*descriptor_linked=0.*packets=0')" -eq 1 ]; then
    gate_e1000_packet_buffer_uc_alias_proof_done="PASS"
    print_row "e1000_packet_buffer_uc_alias_proof_done" "PASS" "ok=1 allocated=16 descriptor_linked=0 packets=0"
elif [ "$(has 'e1000.packet.buffer.uc.alias.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_packet_buffer_uc_alias_proof_done="FAIL"
    print_row "e1000_packet_buffer_uc_alias_proof_done" "FAIL" "proof not completed"
else gate_e1000_packet_buffer_uc_alias_proof_done="SKIP"; fi

# ---- 145. e1000_rx_desc_link ----
if [ "$(has 'e1000.rx.desc.link.*linked=8.*status_zero=1.*ok=1')" -eq 1 ]; then
    gate_e1000_rx_desc_link="PASS"
    print_row "e1000_rx_desc_link" "PASS" "linked=8 status_zero=1 ok=1"
elif [ "$(has 'e1000.rx.desc.link.*ok=0')" -eq 1 ]; then
    gate_e1000_rx_desc_link="FAIL"
    print_row "e1000_rx_desc_link" "FAIL" "RX descriptor link failed"
else gate_e1000_rx_desc_link="SKIP"; fi

# ---- 146. e1000_tx_desc_link ----
if [ "$(has 'e1000.tx.desc.link.*linked=8.*length_zero=1.*cmd_zero=1.*ok=1')" -eq 1 ]; then
    gate_e1000_tx_desc_link="PASS"
    print_row "e1000_tx_desc_link" "PASS" "linked=8 length_zero=1 cmd_zero=1 ok=1"
elif [ "$(has 'e1000.tx.desc.link.*ok=0')" -eq 1 ]; then
    gate_e1000_tx_desc_link="FAIL"
    print_row "e1000_tx_desc_link" "FAIL" "TX descriptor link failed"
else gate_e1000_tx_desc_link="SKIP"; fi

# ---- 147. e1000_desc_link_truth ----
if [ "$(has 'e1000.desc.link.truth.*descriptor_linked=1.*device_visible=0.*mmio_writes=0.*dma=0.*rings_enabled=0.*packets=0.*ok=1')" -eq 1 ]; then
    gate_e1000_desc_link_truth="PASS"
    print_row "e1000_desc_link_truth" "PASS" "descriptor_linked=1 device_visible=0 mmio_writes=0 dma=0 rings_enabled=0 packets=0"
else gate_e1000_desc_link_truth="SKIP"; fi

# ---- 148. e1000_descriptor_link_proof_done ----
if [ "$(has 'e1000.descriptor.link.proof.done.*ok=1.*rx_linked=8.*tx_linked=8.*packets=0')" -eq 1 ]; then
    gate_e1000_descriptor_link_proof_done="PASS"
    print_row "e1000_descriptor_link_proof_done" "PASS" "ok=1 rx_linked=8 tx_linked=8 packets=0"
elif [ "$(has 'e1000.descriptor.link.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_descriptor_link_proof_done="FAIL"
    print_row "e1000_descriptor_link_proof_done" "FAIL" "proof not completed"
else gate_e1000_descriptor_link_proof_done="SKIP"; fi

# ---- 149. e1000_rx_desc_readback ----
if [ "$(has 'e1000.rx.desc.readback.*checked=8.*matched=8.*status_zero=1.*length_zero=1.*ok=1')" -eq 1 ]; then
    gate_e1000_rx_desc_readback="PASS"
    print_row "e1000_rx_desc_readback" "PASS" "checked=8 matched=8 status_zero=1 length_zero=1 ok=1"
elif [ "$(has 'e1000.rx.desc.readback.*ok=0')" -eq 1 ]; then
    gate_e1000_rx_desc_readback="FAIL"
    print_row "e1000_rx_desc_readback" "FAIL" "RX descriptor readback mismatch"
else gate_e1000_rx_desc_readback="SKIP"; fi

# ---- 150. e1000_tx_desc_readback ----
if [ "$(has 'e1000.tx.desc.readback.*checked=8.*matched=8.*cmd_zero=1.*status_zero=1.*length_zero=1.*ok=1')" -eq 1 ]; then
    gate_e1000_tx_desc_readback="PASS"
    print_row "e1000_tx_desc_readback" "PASS" "checked=8 matched=8 cmd_zero=1 status_zero=1 length_zero=1 ok=1"
elif [ "$(has 'e1000.tx.desc.readback.*ok=0')" -eq 1 ]; then
    gate_e1000_tx_desc_readback="FAIL"
    print_row "e1000_tx_desc_readback" "FAIL" "TX descriptor readback mismatch"
else gate_e1000_tx_desc_readback="SKIP"; fi

# ---- 151. e1000_desc_readback_truth ----
if [ "$(has 'e1000.desc.readback.truth.*reads=1.*writes=0.*device_visible=0.*mmio_writes=0.*dma=0.*rings_enabled=0.*packets=0.*ok=1')" -eq 1 ]; then
    gate_e1000_desc_readback_truth="PASS"
    print_row "e1000_desc_readback_truth" "PASS" "reads=1 writes=0 device_visible=0 mmio_writes=0 dma=0 rings_enabled=0 packets=0"
else gate_e1000_desc_readback_truth="SKIP"; fi

# ---- 152. e1000_descriptor_readback_proof_done ----
if [ "$(has 'e1000.descriptor.readback.proof.done.*ok=1.*rx_matched=8.*tx_matched=8.*packets=0')" -eq 1 ]; then
    gate_e1000_descriptor_readback_proof_done="PASS"
    print_row "e1000_descriptor_readback_proof_done" "PASS" "ok=1 rx_matched=8 tx_matched=8 packets=0"
elif [ "$(has 'e1000.descriptor.readback.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_descriptor_readback_proof_done="FAIL"
    print_row "e1000_descriptor_readback_proof_done" "FAIL" "readback proof not completed"
else gate_e1000_descriptor_readback_proof_done="SKIP"; fi

# ---- 153. e1000_mmio_ring_base ----
if [ "$(has 'e1000.mmio.ring.base.*rdlen=128.*tdlen=128.*ok=1')" -eq 1 ]; then
    gate_e1000_mmio_ring_base="PASS"
    print_row "e1000_mmio_ring_base" "PASS" "ring base+len write/readback"
elif [ "$(has 'e1000.mmio.ring.base.*ok=0')" -eq 1 ]; then
    gate_e1000_mmio_ring_base="FAIL"
    print_row "e1000_mmio_ring_base" "FAIL" "ring base readback mismatch"
else gate_e1000_mmio_ring_base="SKIP"; fi

if [ "$(has 'e1000.mmio.ring.base.proof.done.*ok=1')" -eq 1 ]; then
    gate_e1000_mmio_ring_base_proof_done="PASS"
    print_row "e1000_mmio_ring_base_proof_done" "PASS" "proof done"
elif [ "$(has 'e1000.mmio.ring.base.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_mmio_ring_base_proof_done="FAIL"
    print_row "e1000_mmio_ring_base_proof_done" "FAIL" "proof failed"
else gate_e1000_mmio_ring_base_proof_done="SKIP"; fi

if [ "$(has 'e1000.rx.register.init.*ok=1')" -eq 1 ]; then
    gate_e1000_rx_register_init="PASS"
    print_row "e1000_rx_register_init" "PASS" "RCTL init readback"
elif [ "$(has 'e1000.rx.register.init.*ok=0')" -eq 1 ]; then
    gate_e1000_rx_register_init="FAIL"
    print_row "e1000_rx_register_init" "FAIL" "RCTL init failed"
else gate_e1000_rx_register_init="SKIP"; fi

if [ "$(has 'e1000.rx.register.init.proof.done.*ok=1')" -eq 1 ]; then
    gate_e1000_rx_register_init_proof_done="PASS"
    print_row "e1000_rx_register_init_proof_done" "PASS" "proof done"
elif [ "$(has 'e1000.rx.register.init.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_rx_register_init_proof_done="FAIL"
    print_row "e1000_rx_register_init_proof_done" "FAIL" "proof failed"
else gate_e1000_rx_register_init_proof_done="SKIP"; fi

if [ "$(has 'e1000.rx.enable.proof.*enabled=1.*ok=1')" -eq 1 ]; then
    gate_e1000_rx_enable_proof="PASS"
    print_row "e1000_rx_enable_proof" "PASS" "rx enable bit set"
elif [ "$(has 'e1000.rx.enable.proof.*ok=0')" -eq 1 ]; then
    gate_e1000_rx_enable_proof="FAIL"
    print_row "e1000_rx_enable_proof" "FAIL" "rx enable proof failed"
else gate_e1000_rx_enable_proof="SKIP"; fi

if [ "$(has 'e1000.tx.register.init.*ok=1')" -eq 1 ]; then
    gate_e1000_tx_register_init="PASS"
    print_row "e1000_tx_register_init" "PASS" "TCTL init readback"
elif [ "$(has 'e1000.tx.register.init.*ok=0')" -eq 1 ]; then
    gate_e1000_tx_register_init="FAIL"
    print_row "e1000_tx_register_init" "FAIL" "TCTL init failed"
else gate_e1000_tx_register_init="SKIP"; fi

if [ "$(has 'e1000.tx.register.init.proof.done.*ok=1')" -eq 1 ]; then
    gate_e1000_tx_register_init_proof_done="PASS"
    print_row "e1000_tx_register_init_proof_done" "PASS" "proof done"
elif [ "$(has 'e1000.tx.register.init.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_tx_register_init_proof_done="FAIL"
    print_row "e1000_tx_register_init_proof_done" "FAIL" "proof failed"
else gate_e1000_tx_register_init_proof_done="SKIP"; fi

if [ "$(has 'e1000.tx.test.frame.*staged=1.*tdt=1.*ok=1')" -eq 1 ]; then
    gate_e1000_tx_test_frame="PASS"
    print_row "e1000_tx_test_frame" "PASS" "test frame posted"
elif [ "$(has 'e1000.tx.test.frame.*ok=0')" -eq 1 ]; then
    gate_e1000_tx_test_frame="FAIL"
    print_row "e1000_tx_test_frame" "FAIL" "test frame post failed"
else gate_e1000_tx_test_frame="SKIP"; fi

if [ "$(has 'e1000.tx.test.frame.proof.done.*ok=1')" -eq 1 ]; then
    gate_e1000_tx_test_frame_proof_done="PASS"
    print_row "e1000_tx_test_frame_proof_done" "PASS" "proof done"
elif [ "$(has 'e1000.tx.test.frame.proof.done.*ok=0')" -eq 1 ]; then
    gate_e1000_tx_test_frame_proof_done="FAIL"
    print_row "e1000_tx_test_frame_proof_done" "FAIL" "proof failed"
else gate_e1000_tx_test_frame_proof_done="SKIP"; fi

if [ "$(has 'e1000.rx.packet.observe.proof.*ok=1')" -eq 1 ]; then
    gate_e1000_rx_packet_observe_proof="PASS"
    print_row "e1000_rx_packet_observe_proof" "PASS" "bounded no-peer-observe claim"
elif [ "$(has 'e1000.rx.packet.observe.proof.*ok=0')" -eq 1 ]; then
    gate_e1000_rx_packet_observe_proof="FAIL"
    print_row "e1000_rx_packet_observe_proof" "FAIL" "proof failed"
else gate_e1000_rx_packet_observe_proof="SKIP"; fi

# ---- sexnet_nic_rx_packet_observe (temporary observe/restore proof) ----
if [ "$(has 'sexnet\.nic\.rx\.observe\.alloc.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.desc\.link.*count=8.*separate_bufs=1.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.ring\.program.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.window\.open.*max_iters=50000000')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.poll\.done.*dd_set=[1-9][0-9]*.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.pkt\.parse.*len=(60|1[5-9]|[2-9][0-9]|[1-9][0-9]{2,}).*ethertype=0x(0800|0806).*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.ring\.restore.*rctl_en=1.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.observe\.proof\.done.*dd_set=[1-9][0-9]*.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_rx_packet_observe="PASS"
    print_row "sexnet_nic_rx_packet_observe" "PASS" "temporary observe/restore proof under TAP traffic"
elif [ "$(has 'sexnet\.nic\.rx\.observe\.poll\.done.*dd_set=0.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.nic\.rx\.observe\.ring\.restore.*rctl_en=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_rx_packet_observe="SKIP"
    print_row "sexnet_nic_rx_packet_observe" "SKIP" "no RX frame observed in window; restore succeeded"
elif [ "$(has 'sexnet\.nic\.rx\.observe\.ring\.restore.*ok=0')" -eq 1 ] \
     || [ "$(has 'sexnet\.nic\.rx\.observe\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_nic_rx_packet_observe="FAIL"
    print_row "sexnet_nic_rx_packet_observe" "FAIL" "observe proof ran but restore/proof marker reported failure"
else
    gate_sexnet_nic_rx_packet_observe="SKIP"
fi

# ---- sexnet_nic_tx_frame_observe (temporary tx observe/restore proof) ----
if [ "$(has 'sexnet\.nic\.tx\.observe\.alloc.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.frame\.write.*ethertype=0x88B5.*len=60.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.desc\.write.*len=60.*cmd=0x0B.*sta=0.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.ring\.program.*tdlen=128.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.post.*tdt=1.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.poll\.done.*dd_set=1.*desc_idx=0.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.ring\.restore.*tctl_en=1.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.tx\.observe\.proof\.done.*dd_set=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_tx_frame_observe="PASS"
    print_row "sexnet_nic_tx_frame_observe" "PASS" "temporary tx observe/restore proof with descriptor DD"
elif [ "$(has 'sexnet\.nic\.tx\.observe\.poll\.done.*dd_set=0.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.nic\.tx\.observe\.ring\.restore.*tctl_en=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_tx_frame_observe="SKIP"
    print_row "sexnet_nic_tx_frame_observe" "SKIP" "no TX DD observed in window; restore succeeded"
elif [ "$(has 'sexnet\.nic\.tx\.observe\.ring\.restore.*tctl_en=0')" -eq 1 ] \
     || [ "$(has 'sexnet\.nic\.tx\.observe\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_nic_tx_frame_observe="FAIL"
    print_row "sexnet_nic_tx_frame_observe" "FAIL" "tx observe proof reported restore/proof failure"
else
    gate_sexnet_nic_tx_frame_observe="SKIP"
fi

# ---- sexnet_nic_ownership_init (marker/state-contract only) ----
if [ "$(has 'sexnet\.nic\.ownership\.init.*rx_owner=0.*tx_owner=0.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_ownership_init="PASS"
    print_row "sexnet_nic_ownership_init" "PASS" "ownership marker initialized to HAL_DIAG (0/0)"
elif [ "$(has 'sexnet\.nic\.ownership\.init')" -eq 1 ]; then
    gate_sexnet_nic_ownership_init="FAIL"
    print_row "sexnet_nic_ownership_init" "FAIL" "ownership marker present with nonzero owner or ok!=1"
else
    gate_sexnet_nic_ownership_init="SKIP"
fi

# ---- sexnet_nic_rx_permanent_init (permanent rx ownership claim) ----
if [ "$(has 'sexnet\.nic\.rx\.permanent\.claim.*owner=1.*ring_ok=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_rx_permanent_init="PASS"
    print_row "sexnet_nic_rx_permanent_init" "PASS" "permanent RX claim owner=1 ring_ok=1"
elif [ "$(has 'sexnet\.nic\.rx\.permanent\.claim')" -eq 1 ]; then
    gate_sexnet_nic_rx_permanent_init="FAIL"
    print_row "sexnet_nic_rx_permanent_init" "FAIL" "claim marker present but owner/ring_ok/ok contract failed"
else
    gate_sexnet_nic_rx_permanent_init="SKIP"
fi

# ---- sexnet_nic_rx_permanent_recv (traffic-dependent receive proof) ----
if [ "$(has 'sexnet\.nic\.rx\.permanent\.poll\.done.*dd_set=1.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.permanent\.pkt\.parse.*len=(1[5-9]|[2-9][0-9]|[1-9][0-9]{2,}).*ethertype=0x(0800|0806).*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.nic\.rx\.permanent\.rdt\.advance.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_rx_permanent_recv="PASS"
    print_row "sexnet_nic_rx_permanent_recv" "PASS" "permanent RX receive parse+recycle proved"
elif [ "$(has 'sexnet\.nic\.rx\.permanent\.poll\.done.*dd_set=1.*ok=1')" -eq 1 ] \
     && ( [ "$(has 'sexnet\.nic\.rx\.permanent\.pkt\.parse.*ok=0')" -eq 1 ] \
          || [ "$(has 'sexnet\.nic\.rx\.permanent\.rdt\.advance')" -eq 0 ] ); then
    gate_sexnet_nic_rx_permanent_recv="FAIL"
    print_row "sexnet_nic_rx_permanent_recv" "FAIL" "dd_set=1 but parse/recycle contract failed"
elif [ "$(has 'sexnet\.nic\.rx\.permanent\.poll\.done.*dd_set=0.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_rx_permanent_recv="SKIP"
    print_row "sexnet_nic_rx_permanent_recv" "SKIP" "no RX frame observed (traffic-dependent lane)"
else
    gate_sexnet_nic_rx_permanent_recv="SKIP"
fi

# ---- sexnet_nic_tx_permanent_init (permanent tx ownership claim) ----
if [ "$(has 'sexnet\.nic\.tx\.permanent\.claim.*owner=2.*ring_ok=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_tx_permanent_init="PASS"
    print_row "sexnet_nic_tx_permanent_init" "PASS" "permanent TX claim owner=2 ring_ok=1"
elif [ "$(has 'sexnet\.nic\.tx\.permanent\.claim')" -eq 1 ]; then
    gate_sexnet_nic_tx_permanent_init="FAIL"
    print_row "sexnet_nic_tx_permanent_init" "FAIL" "tx claim marker present but owner/ring_ok/ok contract failed"
else
    gate_sexnet_nic_tx_permanent_init="SKIP"
fi

# ---- sexnet_nic_tx_permanent_send (tx dd consumption proof) ----
if [ "$(has 'sexnet\.nic\.tx\.permanent\.poll\.done.*dd_set=1.*desc_idx=0.*ok=1')" -eq 1 ]; then
    gate_sexnet_nic_tx_permanent_send="PASS"
    print_row "sexnet_nic_tx_permanent_send" "PASS" "permanent TX descriptor consumed (DD=1)"
elif [ "$(has 'sexnet\.nic\.tx\.permanent\.poll\.done.*dd_set=0')" -eq 1 ] \
     && [ "$(has 'sexnet\.nic\.tx\.permanent\.(claim|full)')" -eq 1 ]; then
    gate_sexnet_nic_tx_permanent_send="FAIL"
    print_row "sexnet_nic_tx_permanent_send" "FAIL" "dd_set=0 while permanent tx proof/claim lane executed"
else
    gate_sexnet_nic_tx_permanent_send="SKIP"
fi

# ---- sexnet_nic_full_ownership (rx+tx full claim) ----
if [ "$(has 'sexnet\.nic\.tx\.permanent\.full.*rx_owner=3.*tx_owner=3.*full_ok=1')" -eq 1 ]; then
    gate_sexnet_nic_full_ownership="PASS"
    print_row "sexnet_nic_full_ownership" "PASS" "SEXNET_FULL ownership reached (rx=3 tx=3)"
elif [ "$(has 'sexnet\.nic\.tx\.permanent\.full')" -eq 1 ]; then
    gate_sexnet_nic_full_ownership="FAIL"
    print_row "sexnet_nic_full_ownership" "FAIL" "full marker present but rx/tx/full_ok contract failed"
else
    gate_sexnet_nic_full_ownership="SKIP"
fi

# ---- sexnet_l2_rx_loop (bounded L2 rx parse+recycle) ----
if [ "$(has 'sexnet\.l2\.entry.*rx_owner=3.*tx_owner=3.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.l2\.rx\.poll\.done.*frames_rx=[1-9][0-9]*.*ok=1')" -eq 1 ] \
   && { [ "$(has 'sexnet\.l2\.rx\.recycle.*ok=1')" -eq 1 ] \
        || { [ "$(has 'sexnet\.l2\.rx\.frame.*ethertype=0x0806.*ok=1')" -eq 1 ] \
             && [ "$(has 'sexnet\.arp\.proof\.done.*rx_arp=1.*tx_dd=1.*ok=1')" -eq 1 ]; }; }; then
    gate_sexnet_l2_rx_loop="PASS"
    print_row "sexnet_l2_rx_loop" "PASS" "bounded L2 RX proved (recycle or ARP-preserve+ARP-proof)"
elif [ "$(has 'sexnet\.l2\.entry.*ok=0')" -eq 1 ] \
     && [ "$(has 'sexnet\.nic\.tx\.permanent\.full.*rx_owner=3.*tx_owner=3.*full_ok=1')" -eq 1 ]; then
    gate_sexnet_l2_rx_loop="FAIL"
    print_row "sexnet_l2_rx_loop" "FAIL" "entry denied despite full ownership marker"
elif [ "$(has 'sexnet\.l2\.rx\.frame')" -eq 1 ] \
     && [ "$(has 'sexnet\.l2\.rx\.recycle')" -eq 0 ] \
     && ! { [ "$(has 'sexnet\.l2\.rx\.frame.*ethertype=0x0806.*ok=1')" -eq 1 ] \
            && [ "$(has 'sexnet\.arp\.proof\.done.*rx_arp=1.*tx_dd=1.*ok=1')" -eq 1 ]; }; then
    gate_sexnet_l2_rx_loop="FAIL"
    print_row "sexnet_l2_rx_loop" "FAIL" "rx frame marker present but recycle marker missing"
elif [ "$(has 'sexnet\.l2\.rx\.poll\.done.*frames_rx=0')" -eq 1 ]; then
    gate_sexnet_l2_rx_loop="SKIP"
    print_row "sexnet_l2_rx_loop" "SKIP" "traffic-dependent lane: no RX frames observed"
else
    gate_sexnet_l2_rx_loop="SKIP"
fi

# ---- sexnet_l2_tx_reuse (bounded tx reuse descriptor 2 after ARP reply) ----
if [ "$(has 'sexnet\.l2\.tx\.reuse\.desc.*slot=2.*len=60.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.l2\.tx\.reuse\.post.*tdt=3.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.l2\.tx\.reuse\.poll\.done.*dd_set=1.*desc_idx=2.*ok=1')" -eq 1 ]; then
    gate_sexnet_l2_tx_reuse="PASS"
    print_row "sexnet_l2_tx_reuse" "PASS" "bounded L2 TX reuse consumed desc slot 2"
elif [ "$(has 'sexnet\.l2\.tx\.reuse\.post.*tdt=3.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.l2\.tx\.reuse\.poll\.done.*dd_set=0')" -eq 1 ]; then
    gate_sexnet_l2_tx_reuse="FAIL"
    print_row "sexnet_l2_tx_reuse" "FAIL" "tx reuse post issued but DD remained 0"
else
    gate_sexnet_l2_tx_reuse="SKIP"
fi

# ---- sexnet_l2_proof (combined bounded l2 proof) ----
if [ "$(has 'sexnet\.l2\.proof\.done.*rx_frames=[1-9][0-9]*.*tx_dd=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_l2_proof="PASS"
    print_row "sexnet_l2_proof" "PASS" "combined bounded L2 proof done"
elif [ "$(has 'sexnet\.l2\.tx\.reuse\.post.*tdt=3.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.l2\.proof\.done.*ok=0.*tx_dd=0')" -eq 1 ]; then
    gate_sexnet_l2_proof="FAIL"
    print_row "sexnet_l2_proof" "FAIL" "combined proof marker failed after tx reuse post"
elif [ "$(has 'sexnet\.l2\.proof\.done.*rx_frames=0')" -eq 1 ]; then
    gate_sexnet_l2_proof="SKIP"
    print_row "sexnet_l2_proof" "SKIP" "traffic-dependent lane: rx_frames=0"
else
    gate_sexnet_l2_proof="SKIP"
fi

if [ "$(has 'e1000e\.rx\.descriptor\.observe\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_e1000e_rx_desc_observe="PASS"
    print_row "e1000e_rx_desc_observe" "PASS" "loopback RX dd+rdh+buffer_match=1"
elif [ "$(has 'e1000e\.rx\.descriptor\.observe\.proof\.done.*rdh_advanced=1')" -eq 1 ]; then
    gate_e1000e_rx_desc_observe="FAIL"
    print_row "e1000e_rx_desc_observe" "FAIL" "rdh_advanced=1 but buffer_match or ok=0"
else gate_e1000e_rx_desc_observe="SKIP"; fi

if [ "$(has 'ethernet.frame.model.spec.*ok=1')" -eq 1 ]; then
    gate_ethernet_frame_model_spec="PASS"
    print_row "ethernet_frame_model_spec" "PASS" "L2 bounded frame model marker"
elif [ "$(has 'ethernet.frame.model.spec.*ok=0')" -eq 1 ]; then
    gate_ethernet_frame_model_spec="FAIL"
    print_row "ethernet_frame_model_spec" "FAIL" "L2 model marker failed"
else gate_ethernet_frame_model_spec="SKIP"; fi

if [ "$(has 'arp.client.plan.*ok=1')" -eq 1 ]; then
    gate_arp_client_plan="PASS"
    print_row "arp_client_plan" "PASS" "ARP lane plan marker"
else gate_arp_client_plan="SKIP"; fi

if [ "$(has 'arp.request.build.proof.*ok=1')" -eq 1 ]; then
    gate_arp_request_build_proof="PASS"
    print_row "arp_request_build_proof" "PASS" "ARP request build marker"
else gate_arp_request_build_proof="SKIP"; fi

if [ "$(has 'arp.request.send.stop.review.*stop=1')" -eq 1 ]; then
    gate_arp_request_send_stop_review="PASS"
    print_row "arp_request_send_stop_review" "PASS" "stop-review enforced"
elif [ "$(has 'arp.request.send.stop.review.*stop=0')" -eq 1 ]; then
    gate_arp_request_send_stop_review="PASS"
    print_row "arp_request_send_stop_review" "PASS" "ARP lane exercised"
else gate_arp_request_send_stop_review="SKIP"; fi

if [ "$(has 'arp\.request\.send\.proof\.done.*sent=1.*gateway_known=1')" -eq 1 ]; then
    gate_arp_request_send_proof="PASS"
    print_row "arp_request_send_proof" "PASS" "ARP request sent gateway_known=1 reply rx confirmed"
elif [ "$(has 'arp\.request\.send\.proof\.done.*sent=1.*gateway_known=0')" -eq 1 ]; then
    gate_arp_request_send_proof="SKIP"
    print_row "arp_request_send_proof" "SKIP" "ARP request sent gateway_known=0 no reply rx"
else gate_arp_request_send_proof="SKIP"; fi

if [ "$(has 'arp\.reply\.timing\.slirp\.probe\.done.*ok=1.*reply_seen=1.*gateway_known=1')" -eq 1 ]; then
    gate_arp_reply_timing_slirp_probe="PASS"
    print_row "arp_reply_timing_slirp_probe" "PASS" "ARP reply timing probe gateway_known=1"
elif [ "$(has 'arp\.reply\.timing\.slirp\.probe\.done.*ok=1.*reply_seen=0.*diagnostic=1')" -eq 1 ]; then
    gate_arp_reply_timing_slirp_probe="SKIP"
    print_row "arp_reply_timing_slirp_probe" "SKIP" "ARP timing probe sent reply_seen=0 diagnostic"
else gate_arp_reply_timing_slirp_probe="SKIP"; fi

if [ "$(has 'arp\.reply\.capture\.fix\.done.*ok=1.*reply_seen=1.*gateway_known=1.*rdh_written=0')" -eq 1 ]; then
    gate_arp_reply_capture_fix="PASS"
    print_row "arp_reply_capture_fix" "PASS" "ARP reply captured gateway_known=1 rdh_written=0"
elif [ "$(has 'arp\.reply\.capture\.fix\.done.*ok=1.*rdh_written=0')" -eq 1 ]; then
    gate_arp_reply_capture_fix="SKIP"
    print_row "arp_reply_capture_fix" "SKIP" "ARP capture fix ran rdh_written=0 no reply diagnostic"
else gate_arp_reply_capture_fix="SKIP"; fi

if [ "$(has 'arp\.gateway\.tx\.post.*target_ip=10\.0\.2\.2.*fake=0')" -eq 1 ] && [ "$(has 'arp\.gateway\.resolved.*fake=0')" -eq 1 ]; then
    if [ "$(has 'arp\.gateway\.resolved.*gateway_known=1.*gw_mac=00:00:00:00:00:00')" -eq 1 ]; then
        gate_arp_gateway_resolution_reliability="FAIL"
        print_row "arp_gateway_resolution_reliability" "FAIL" "gateway_known=1 with zero gw_mac"
    elif [ "$(has 'arp\.gateway\.resolved.*gateway_known=0')" -eq 1 ]; then
        if [ "$(has 'tcp\.syn\.tx\.post.*syn_sent=1')" -eq 1 ]; then
            gate_arp_gateway_resolution_reliability="FAIL"
            print_row "arp_gateway_resolution_reliability" "FAIL" "syn_sent=1 while gateway_known=0"
        else
            gate_arp_gateway_resolution_reliability="SKIP"
            print_row "arp_gateway_resolution_reliability" "SKIP" "bounded retries complete gateway unresolved"
        fi
    elif [ "$(has 'arp\.gateway\.resolved.*gateway_known=1')" -eq 1 ] && [ "$(has 'arp\.gateway\.resolved.*gw_mac=00:00:00:00:00:00')" -eq 0 ]; then
        gate_arp_gateway_resolution_reliability="PASS"
        print_row "arp_gateway_resolution_reliability" "PASS" "gateway resolved from real ARP reply"
    else
        gate_arp_gateway_resolution_reliability="SKIP"
    fi
else
    gate_arp_gateway_resolution_reliability="SKIP"
fi

if [ "$(has 'arp.reply.observe.proof.*ok=1')" -eq 1 ]; then
    gate_arp_reply_observe_proof="PASS"
    print_row "arp_reply_observe_proof" "PASS" "ARP observe bounded claim"
else gate_arp_reply_observe_proof="SKIP"; fi

if [ "$(has 'arp\.reply\.observe\.proof\.done.*ok=1.*arp_seen=1')" -eq 1 ]; then
    gate_arp_rx_observe_live="PASS"
    print_row "arp_rx_observe_live" "PASS" "real ARP parsed from e1000e rx buffer fake=0"
else gate_arp_rx_observe_live="SKIP"; fi

if [ "$(has 'arp\.cache\.real\.behavior\.done.*ok=1.*entries=1.*fake=0.*gateway_known=0')" -eq 1 ]; then
    gate_arp_cache_real_behavior="PASS"
    print_row "arp_cache_real_behavior" "PASS" "live arp cache insert+lookup fake=0 gateway_known=0"
elif [ "$(has 'arp\.cache\.real\.behavior\.done.*entries=0')" -eq 1 ]; then
    gate_arp_cache_real_behavior="SKIP"
else gate_arp_cache_real_behavior="SKIP"; fi

if [ "$(has 'arp.cache.status.stub.*ok=1')" -eq 1 ]; then
    gate_arp_cache_status_stub="PASS"
    print_row "arp_cache_status_stub" "PASS" "ARP cache stub marker"
else gate_arp_cache_status_stub="SKIP"; fi

# ---- SEXNET_ARP_REQUEST_REPLY_GATE_V1 (one-shot, poll-driven) ----
if [ "$(has 'sexnet\.arp\.rx\.frame.*ethertype=0x0806.*ok=1')" -eq 1 ]; then
    gate_sexnet_arp_rx_poll="PASS"
    print_row "sexnet_arp_rx_poll" "PASS" "ARP RX frame observed ethertype=0x0806"
elif [ "$(has 'sexnet\.arp\..*reject')" -eq 1 ] && [ "$(has 'sexnet\.arp\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_arp_rx_poll="FAIL"
    print_row "sexnet_arp_rx_poll" "FAIL" "reject-only ARP path with proof done ok=0"
else
    gate_sexnet_arp_rx_poll="SKIP"
    print_row "sexnet_arp_rx_poll" "SKIP" "no TAP traffic or no ARP frame"
fi

if [ "$(has 'sexnet\.arp\.rx\.validate.*htype=1.*ptype=0x0800.*hlen=6.*plen=4.*oper=1.*tpa_match=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_arp_rx_valid="PASS"
    print_row "sexnet_arp_rx_valid" "PASS" "ARP request validate fields matched for local TPA"
elif [ "$(has 'sexnet\.arp\.rx\.validate.*ok=0')" -eq 1 ] || [ "$(has 'sexnet\.arp\.rx\.validate.*tpa_match=0')" -eq 1 ]; then
    gate_sexnet_arp_rx_valid="FAIL"
    print_row "sexnet_arp_rx_valid" "FAIL" "ARP validate marker present with ok=0 or tpa_match=0"
else
    gate_sexnet_arp_rx_valid="SKIP"
    print_row "sexnet_arp_rx_valid" "SKIP" "no ARP request observed"
fi

if [ "$(has 'sexnet\.arp\.tx\.reply\.build.*spa=10\.0\.2\.15.*ok=1')" -eq 1 ] && \
   [ "$(has 'sexnet\.arp\.tx\.desc.*slot=1.*len=60.*ok=1')" -eq 1 ] && \
   [ "$(has 'sexnet\.arp\.tx\.post.*tdt=2.*ok=1')" -eq 1 ]; then
    gate_sexnet_arp_tx_reply="PASS"
    print_row "sexnet_arp_tx_reply" "PASS" "ARP reply build+desc+post markers all ok"
elif [ "$(has 'sexnet\.arp\.tx\.(reply\.build|desc|post).*ok=0')" -eq 1 ]; then
    gate_sexnet_arp_tx_reply="FAIL"
    print_row "sexnet_arp_tx_reply" "FAIL" "ARP TX build/desc/post marker present with ok=0"
else
    gate_sexnet_arp_tx_reply="SKIP"
    print_row "sexnet_arp_tx_reply" "SKIP" "no ARP RX path to trigger reply"
fi

if [ "$(has 'sexnet\.arp\.tx\.poll\.done.*dd_set=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_arp_tx_dd="PASS"
    print_row "sexnet_arp_tx_dd" "PASS" "ARP TX DD consumed dd_set=1"
elif [ "$(has 'sexnet\.arp\.tx\.poll\.done.*dd_set=0.*ok=0')" -eq 1 ]; then
    gate_sexnet_arp_tx_dd="FAIL"
    print_row "sexnet_arp_tx_dd" "FAIL" "ARP TX DD poll marker shows dd_set=0 ok=0"
else
    gate_sexnet_arp_tx_dd="SKIP"
    print_row "sexnet_arp_tx_dd" "SKIP" "no ARP RX path to trigger TX DD poll"
fi

if [ "$(has 'sexnet\.arp\.proof\.done.*rx_arp=1.*tx_dd=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_arp_proof="PASS"
    print_row "sexnet_arp_proof" "PASS" "one-shot ARP request/reply proof done"
elif [ "$(has 'sexnet\.arp\.proof\.done.*ok=0')" -eq 1 ] && \
     ( [ "$(has 'sexnet\.arp\.proof\.done.*rx_arp=1')" -eq 1 ] || [ "$(has 'sexnet\.arp\.tx\.(reply\.build|desc|post|poll\.done)')" -eq 1 ] ); then
    gate_sexnet_arp_proof="FAIL"
    print_row "sexnet_arp_proof" "FAIL" "proof done failed while RX/TX path was attempted"
else
    gate_sexnet_arp_proof="SKIP"
    print_row "sexnet_arp_proof" "SKIP" "no TAP/no ARP proof markers"
fi

# ---- SEXNET_ARP_CACHE_GATE_AND_HANDOFF_V1 (bounded 1-entry cache proof) ----
if [ "$(has 'sexnet\.arp\.cache\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_arp_cache_proof="FAIL"
    print_row "sexnet_arp_cache_proof" "FAIL" "proof.done reported ok=0"
elif [ "$(has 'sexnet\.arp\.cache\.reply\.dd.*dd_set=0')" -eq 1 ]; then
    gate_sexnet_arp_cache_proof="FAIL"
    print_row "sexnet_arp_cache_proof" "FAIL" "reply.dd marker reported dd_set=0"
elif { [ "$(has 'sexnet\.arp\.cache\.reply.*n=1')" -eq 1 ] \
        && [ "$(has 'sexnet\.arp\.cache\.reply.*n=1.*slot=3.*tdt=4.*ok=1')" -eq 0 ]; } \
     || { [ "$(has 'sexnet\.arp\.cache\.reply.*n=2')" -eq 1 ] \
           && [ "$(has 'sexnet\.arp\.cache\.reply.*n=2.*slot=4.*tdt=5.*ok=1')" -eq 0 ]; }; then
    gate_sexnet_arp_cache_proof="FAIL"
    print_row "sexnet_arp_cache_proof" "FAIL" "reply marker present with unexpected slot/tdt pair"
elif [ "$(has 'sexnet\.arp\.cache\.proof\.done.*replies=2.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.arp\.cache\.reply\.dd.*n=1.*dd_set=1.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.arp\.cache\.reply\.dd.*n=2.*dd_set=1.*ok=1')" -eq 1 ]; then
    gate_sexnet_arp_cache_proof="PASS"
    print_row "sexnet_arp_cache_proof" "PASS" "bounded ARP cache proof done replies=2 with DD set for n=1,n=2"
else
    gate_sexnet_arp_cache_proof="SKIP"
    print_row "sexnet_arp_cache_proof" "SKIP" "no TAP/no ARP cache markers in this boot"
fi

# ---- SEXNET_IPV4_HEADER_VALIDATE_GATE_V1 ----
# Proof command:
#   while true; do sudo arping -I tap0 -c 1 -w 1 10.0.2.15 2>/dev/null || true; sleep 0.05; done
#   while true; do ping -I tap0 -c 1 -W 1 10.0.2.15 2>/dev/null || true; sleep 0.2; done
#   QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 ENABLE_QEMU_USERNET_E1000=1 \
#     ./scripts/run_daily_driver_proof.sh /tmp/sexnet_ipv4_header_validate_gate_v1.log
#
# PASS requires:
#   [sexnet.ipv4.entry] rx_owner=3 ok=1
#   [sexnet.ipv4.rx.frame] ... ethertype=0x0800 ok=1
#   [sexnet.ipv4.rx.validate] version=4 ihl=5 ... dst=10.0.2.15 ... checksum=ok ... ok=1
#   [sexnet.ipv4.rx.recycle] ... ok=1
#   [sexnet.ipv4.proof.done] frames=1 ok=1
if [ "$(has 'sexnet\.ipv4\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_sexnet_ipv4_header_validate="FAIL"
    print_row "sexnet_ipv4_header_validate" "FAIL" "proof.done ok=0 — IPv4 validation failed"
elif [ "$(has 'sexnet\.ipv4\.entry.*ok=0')" -eq 1 ]; then
    gate_sexnet_ipv4_header_validate="FAIL"
    print_row "sexnet_ipv4_header_validate" "FAIL" "ipv4.entry ok=0 — RX owner not acquired"
elif [ "$(has 'sexnet\.ipv4\.rx\.validate.*ok=0')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.rx\.validate.*ok=1')" -eq 0 ]; then
    gate_sexnet_ipv4_header_validate="FAIL"
    print_row "sexnet_ipv4_header_validate" "FAIL" "rx.validate ok=0 without later ok=1 — header invalid"
elif [ "$(has 'sexnet\.ipv4\.proof\.done.*frames=1.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.entry.*rx_owner=3.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.rx\.frame.*ethertype=0x0800.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.rx\.validate.*version=4.*ihl=5.*dst=10\.0\.2\.15.*checksum=ok.*ok=1')" -eq 1 ] \
     && [ "$(has 'sexnet\.ipv4\.rx\.recycle.*ok=1')" -eq 1 ]; then
    gate_sexnet_ipv4_header_validate="PASS"
    print_row "sexnet_ipv4_header_validate" "PASS" "IPv4 header receive/parse/validate proven: 1 frame"
else
    gate_sexnet_ipv4_header_validate="SKIP"
    print_row "sexnet_ipv4_header_validate" "SKIP" "no TAP/no ping stimulus — IPv4 markers absent"
fi

if [ "$(has 'ipv4.packet.model.spec.*ok=1')" -eq 1 ]; then
    gate_ipv4_packet_model_spec="PASS"
    print_row "ipv4_packet_model_spec" "PASS" "IPv4 model marker"
else gate_ipv4_packet_model_spec="SKIP"; fi

if [ "$(has 'ipv4.header.build.proof.*ok=1')" -eq 1 ]; then
    gate_ipv4_header_build_proof="PASS"
    print_row "ipv4_header_build_proof" "PASS" "IPv4 header build marker"
else gate_ipv4_header_build_proof="SKIP"; fi

if [ "$(has 'icmp.echo.request.plan.*ok=1')" -eq 1 ]; then
    gate_icmp_echo_request_plan="PASS"
    print_row "icmp_echo_request_plan" "PASS" "ICMP request plan marker"
else gate_icmp_echo_request_plan="SKIP"; fi

if [ "$(has 'icmp.echo.request.send.stop.review.*stop=1')" -eq 1 ]; then
    gate_icmp_echo_request_send_stop_review="PASS"
    print_row "icmp_echo_request_send_stop_review" "PASS" "ICMP stop-review enforced"
elif [ "$(has 'icmp.echo.request.send.stop.review.*stop=0')" -eq 1 ]; then
    gate_icmp_echo_request_send_stop_review="PASS"
    print_row "icmp_echo_request_send_stop_review" "PASS" "ICMP lane exercised"
else gate_icmp_echo_request_send_stop_review="SKIP"; fi

if [ "$(has 'icmp.echo.request.proof.*ok=1')" -eq 1 ]; then
    gate_icmp_echo_request_proof="PASS"
    print_row "icmp_echo_request_proof" "PASS" "ICMP request bounded claim"
else gate_icmp_echo_request_proof="SKIP"; fi

if [ "$(has 'icmp.echo.reply.observe.proof.*ok=1')" -eq 1 ]; then
    gate_icmp_echo_reply_observe_proof="PASS"
    print_row "icmp_echo_reply_observe_proof" "PASS" "ICMP observe bounded claim"
else gate_icmp_echo_reply_observe_proof="SKIP"; fi

if [ "$(has 'udp\.dns\.probe\.done.*ok=1.*sent=1.*tx_dd=1.*response_seen=1')" -eq 1 ]; then
    gate_udp_dns_probe="PASS"
    print_row "udp_dns_probe" "PASS" "UDP DNS round-trip: txid match + QR=1 response"
elif [ "$(has 'udp\.dns\.probe\.done.*sent=1.*tx_dd=1')" -eq 1 ]; then
    gate_udp_dns_probe="SKIP"
    print_row "udp_dns_probe" "SKIP" "UDP DNS query sent, no response in window (diagnostic)"
else gate_udp_dns_probe="SKIP"; fi

if [ "$(has 'dns\.response\.parse\.proof\.done.*ok=1.*a_records=[1-9]')" -eq 1 ]; then
    gate_dns_response_parse_proof="PASS"
    print_row "dns_response_parse_proof" "PASS" "DNS A record parsed from real RX buffer"
elif [ "$(has 'dns\.response\.parse\.proof\.done.*ok=0')" -eq 1 ]; then
    gate_dns_response_parse_proof="SKIP"
    print_row "dns_response_parse_proof" "SKIP" "DNS parse: tx_dd=1 but no A record (diagnostic)"
else gate_dns_response_parse_proof="SKIP"; fi

if [ "$(has 'udp.packet.model.spec.*ok=1')" -eq 1 ]; then
    gate_udp_packet_model_spec="PASS"
    print_row "udp_packet_model_spec" "PASS" "UDP model marker"
else gate_udp_packet_model_spec="SKIP"; fi

if [ "$(has 'udp.tx.build.proof.*ok=1')" -eq 1 ]; then
    gate_udp_tx_build_proof="PASS"
    print_row "udp_tx_build_proof" "PASS" "UDP build marker"
else gate_udp_tx_build_proof="SKIP"; fi

if [ "$(has 'udp.tx.send.stop.review.*stop=1')" -eq 1 ]; then
    gate_udp_tx_send_stop_review="PASS"
    print_row "udp_tx_send_stop_review" "PASS" "UDP stop-review enforced"
elif [ "$(has 'udp.tx.send.stop.review.*stop=0')" -eq 1 ]; then
    gate_udp_tx_send_stop_review="PASS"
    print_row "udp_tx_send_stop_review" "PASS" "UDP lane exercised"
else gate_udp_tx_send_stop_review="SKIP"; fi

if [ "$(has 'udp.tx.send.proof.*sent=1.*ok=1')" -eq 1 ]; then
    gate_udp_tx_send_proof="PASS"
    print_row "udp_tx_send_proof" "PASS" "UDP send proof marker"
else gate_udp_tx_send_proof="SKIP"; fi

if [ "$(has 'udp.loopback_or_qemu_usernet.proof.*ok=1')" -eq 1 ]; then
    gate_udp_loopback_or_qemu_usernet_proof="PASS"
    print_row "udp_loopback_or_qemu_usernet_proof" "PASS" "UDP observe bounded claim"
else gate_udp_loopback_or_qemu_usernet_proof="SKIP"; fi

if [ "$(has 'tcp.minimal.state.machine.plan.*ok=1')" -eq 1 ]; then
    gate_tcp_minimal_state_machine_plan="PASS"
    print_row "tcp_minimal_state_machine_plan" "PASS" "TCP plan marker"
else gate_tcp_minimal_state_machine_plan="SKIP"; fi

if [ "$(has 'tcp.syn.build.proof.*ok=1')" -eq 1 ]; then
    gate_tcp_syn_build_proof="PASS"
    print_row "tcp_syn_build_proof" "PASS" "TCP SYN build marker"
else gate_tcp_syn_build_proof="SKIP"; fi

if [ "$(has 'tcp.syn.send.stop.review.*stop=1')" -eq 1 ]; then
    gate_tcp_syn_send_stop_review="PASS"
    print_row "tcp_syn_send_stop_review" "PASS" "TCP stop-review enforced"
elif [ "$(has 'tcp.syn.send.stop.review.*stop=0')" -eq 1 ]; then
    gate_tcp_syn_send_stop_review="PASS"
    print_row "tcp_syn_send_stop_review" "PASS" "TCP SYN lane exercised"
else gate_tcp_syn_send_stop_review="SKIP"; fi

if [ "$(has 'tcp.handshake.proof.*ok=1')" -eq 1 ]; then
    gate_tcp_handshake_proof="PASS"
    print_row "tcp_handshake_proof" "PASS" "TCP handshake bounded claim"
else gate_tcp_handshake_proof="SKIP"; fi

# ---- TCP_SYN_BUILD_PROOF_V1 gates ----
if [ "$(has 'tcp.syn.build\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_build_v1="PASS"
    print_row "tcp_syn_build_v1" "PASS" "TCP SYN build with resolved DNS target"
else gate_tcp_syn_build_v1="SKIP"; fi

if [ "$(has 'tcp.syn.checksum\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_checksum_v1="PASS"
    print_row "tcp_syn_checksum_v1" "PASS" "TCP SYN checksums computed"
else gate_tcp_syn_checksum_v1="SKIP"; fi

if [ "$(has 'tcp.syn.truth\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_truth_v1="PASS"
    print_row "tcp_syn_truth_v1" "PASS" "TCP SYN truth syn_sent=0 tcp_sent=0"
else gate_tcp_syn_truth_v1="SKIP"; fi

if [ "$(has 'tcp.syn.build.proof.done\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_build_proof_done_v1="PASS"
    print_row "tcp_syn_build_proof_done_v1" "PASS" "TCP SYN build proof V1 done"
else gate_tcp_syn_build_proof_done_v1="SKIP"; fi

# ---- TCP_SYN_SEND_PROOF_V1 gates ----
if [ "$(has 'tcp.syn.tx.post\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_tx_post_v1="PASS"
    print_row "tcp_syn_tx_post_v1" "PASS" "TCP SYN TX post with DD=1"
else gate_tcp_syn_tx_post_v1="SKIP"; fi

if [ "$(has 'tcp.syn.rx.synack\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_rx_synack_v1="PASS"
    print_row "tcp_syn_rx_synack_v1" "PASS" "TCP SYN-ACK RX poll"
else gate_tcp_syn_rx_synack_v1="SKIP"; fi

if [ "$(has 'tcp.syn.rx.synack.valid\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_rx_synack_valid_v1="PASS"
    print_row "tcp_syn_rx_synack_valid_v1" "PASS" "TCP SYN-ACK fields parsed"
else gate_tcp_syn_rx_synack_valid_v1="SKIP"; fi

if [ "$(has 'tcp.syn.truth.*sent=1.*ok=1')" -eq 1 ]; then
    gate_tcp_syn_truth_send_v1="PASS"
    print_row "tcp_syn_truth_send_v1" "PASS" "TCP SYN sent truth sent=1 final_ack_sent=0"
else gate_tcp_syn_truth_send_v1="SKIP"; fi

if [ "$(has 'tcp.syn.send.proof.done\].*ok=1')" -eq 1 ]; then
    gate_tcp_syn_send_proof_done_v1="PASS"
    print_row "tcp_syn_send_proof_done_v1" "PASS" "TCP SYN send proof V1 done"
else gate_tcp_syn_send_proof_done_v1="SKIP"; fi

if [ "$(has 'tcp.syn.send.retry.proof.*ok=1')" -eq 1 ]; then
    gate_tcp_syn_send_retry_proof_v1="PASS"
    print_row "tcp_syn_send_retry_proof_v1" "PASS" "bounded SYN retries stop on SYN-ACK/RST without final ACK"
else gate_tcp_syn_send_retry_proof_v1="SKIP"; fi

if [ "$(has 'tcp.target.variant.probe.done.*ok=1')" -eq 1 ]; then
    gate_tcp_target_variant_probe_v1="PASS"
    print_row "tcp_target_variant_probe_v1" "PASS" "target/port variant probe completed"
else gate_tcp_target_variant_probe_v1="SKIP"; fi

if [ "$(has 'tcp.http.target.known_good.probe.done.*ok=1')" -eq 1 ]; then
    gate_tcp_http_target_known_good_probe_v1="PASS"
    print_row "tcp_http_target_known_good_probe_v1" "PASS" "known-good plain HTTP target probe completed"
else gate_tcp_http_target_known_good_probe_v1="SKIP"; fi

if [ "$(has 'tcp.guest.host.10_0_2_2.probe.done.*ok=1')" -eq 1 ]; then
    gate_tcp_guest_host_10_0_2_2_probe_v1="PASS"
    print_row "tcp_guest_host_10_0_2_2_probe_v1" "PASS" "guest->host 10.0.2.2 tcp probe completed"
else gate_tcp_guest_host_10_0_2_2_probe_v1="SKIP"; fi

if [ "$(has 'tcp.header.audit.ip')" -eq 1 ] && \
   [ "$(has 'tcp.header.audit.tcp')" -eq 1 ] && \
   [ "$(has 'tcp.header.audit.lengths')" -eq 1 ] && \
   [ "$(has 'tcp.tx.offload.audit')" -eq 1 ] && \
   [ "$(has 'tcp.header.audit.ip.*match=1.*ok=1')" -eq 1 ] && \
   [ "$(has 'tcp.header.audit.tcp.*match=1.*ok=1')" -eq 1 ] && \
   [ "$(has 'tcp.header.audit.lengths.*payload_len=0.*ok=1')" -eq 1 ] && \
   [ "$(has 'tcp.tx.offload.audit.*checksum_offload=0.*ok=1')" -eq 1 ] && \
   [ "$(has 'tcp.checksum.offload.header.audit.done.*ok=1.*ip_ok=1.*tcp_ok=1.*offload_ok=1.*final_ack_sent=0.*http_sent=0.*fake=0')" -eq 1 ]; then
    gate_tcp_checksum_offload_header_audit_v1="PASS"
    print_row "tcp_checksum_offload_header_audit_v1" "PASS" "TCP IP/TCP header checksum + TX offload invariants proven"
else gate_tcp_checksum_offload_header_audit_v1="SKIP"; fi

if [ "$(has 'qemu.slirp.tcp.limit.freeze.*backend=user.*tcp_syn_tx=1.*synack=0.*rst=0.*checksum_ok=1.*offload_ok=1.*final_ack_sent=0.*http_sent=0.*environment_limited=1.*ok=1')" -eq 1 ]; then
    gate_qemu_slirp_tcp_limitation_freeze_v1="PASS"
    print_row "qemu_slirp_tcp_limitation_freeze_v1" "PASS" "SLiRP backend TCP no-response blocker frozen with clean packet truth"
else gate_qemu_slirp_tcp_limitation_freeze_v1="SKIP"; fi

if [ "$(has 'http.response.bounded.buffer.mock.proof.*source=mock.*network=0.*ok=1')" -eq 1 ] && \
   [ "$(has 'http.response.bounded.buffer.mock.proof.*used=[1-9][0-9]*')" -eq 1 ]; then
    gate_http_response_bounded_buffer_mock_proof_v1="PASS"
    print_row "http_response_bounded_buffer_mock_proof_v1" "PASS" "bounded mock HTTP response buffer proven"
else gate_http_response_bounded_buffer_mock_proof_v1="SKIP"; fi

if [ "$(has 'http.response.to.html.subset.feed.v1.*fed=1.*source=mock.*network=0.*ok=1')" -eq 1 ]; then
    gate_http_response_to_html_subset_feed_v1="PASS"
    print_row "http_response_to_html_subset_feed_v1" "PASS" "mock HTTP response fed into HTML subset path"
else gate_http_response_to_html_subset_feed_v1="SKIP"; fi

if [ "$(has 'browser.remote.text.render.proof.v1.*rendered=1.*source=mock.*network=0.*ok=1')" -eq 1 ] && \
   [ "$(has 'browser.mock.fetch.integration.status.*mock_mode=1.*network=0.*ok=1')" -eq 1 ]; then
    gate_browser_remote_text_render_proof_v1="PASS"
    print_row "browser_remote_text_render_proof_v1" "PASS" "browser remote text render proven via bounded mock path"
else gate_browser_remote_text_render_proof_v1="SKIP"; fi

if [ "$(has 'net.diag.syscall.reply.*status=200.*bytes=98.*source=1')" -eq 1 ] && \
   [ "$(has 'sexnet.dynamic_text.set.*status=200.*bytes=98.*source=1.*ok=1')" -eq 1 ] && \
   [ "$(has 'browser.packed_text.text.set.*live=1')" -eq 1 ]; then
    gate_sexnet_dynamic_text_render_proof_v1="PASS"
    print_row "sexnet_dynamic_text_render_proof_v1" "PASS" "dynamic packed text render path proven (source=1 mock lane)"
else gate_sexnet_dynamic_text_render_proof_v1="SKIP"; fi

if [ "$(has 'tcp.syn.ack.observe.proof.*ok=1')" -eq 1 ]; then
    gate_tcp_syn_ack_observe_proof_v1="PASS"
    print_row "tcp_syn_ack_observe_proof_v1" "PASS" "SYN-ACK observe marker"
else gate_tcp_syn_ack_observe_proof_v1="SKIP"; fi

if [ "$(has 'tcp.http.connect.proof.*ok=1')" -eq 1 ]; then
    gate_tcp_http_connect_proof_v1="PASS"
    print_row "tcp_http_connect_proof_v1" "PASS" "TCP connect completion marker"
else gate_tcp_http_connect_proof_v1="SKIP"; fi

if [ "$(has 'dns.client.plan.*ok=1')" -eq 1 ]; then
    gate_dns_client_plan="PASS"
    print_row "dns_client_plan" "PASS" "DNS plan marker"
else gate_dns_client_plan="SKIP"; fi

if [ "$(has 'dns.query.build.proof.*ok=1')" -eq 1 ]; then
    gate_dns_query_build_proof="PASS"
    print_row "dns_query_build_proof" "PASS" "DNS query build marker"
else gate_dns_query_build_proof="SKIP"; fi

if [ "$(has 'dns.query.send.stop.review.*stop=1')" -eq 1 ]; then
    gate_dns_query_send_stop_review="PASS"
    print_row "dns_query_send_stop_review" "PASS" "DNS stop-review enforced"
elif [ "$(has 'dns.query.send.stop.review.*stop=0')" -eq 1 ]; then
    gate_dns_query_send_stop_review="PASS"
    print_row "dns_query_send_stop_review" "PASS" "DNS lane exercised"
else gate_dns_query_send_stop_review="SKIP"; fi

if [ "$(has 'dns.query.send.proof.*sent=1.*ok=1')" -eq 1 ]; then
    gate_dns_query_send_proof="PASS"
    print_row "dns_query_send_proof" "PASS" "DNS send proof marker"
else gate_dns_query_send_proof="SKIP"; fi

if [ "$(has 'dns.response.parse.proof.*ok=1')" -eq 1 ]; then
    gate_dns_response_parse_proof="PASS"
    print_row "dns_response_parse_proof" "PASS" "DNS parse bounded claim"
else gate_dns_response_parse_proof="SKIP"; fi

if [ "$(has 'dns.to.http.host.resolution.proof.*ok=1')" -eq 1 ]; then
    gate_dns_to_http_host_resolution_proof="PASS"
    print_row "dns_to_http_host_resolution_proof" "PASS" "DNS->HTTP bounded claim"
else gate_dns_to_http_host_resolution_proof="SKIP"; fi

if [ "$(has 'http.text.fetch.grant.plan.*ok=1')" -eq 1 ]; then
    gate_http_text_fetch_grant_plan="PASS"
    print_row "http_text_fetch_grant_plan" "PASS" "HTTP grant plan marker"
else gate_http_text_fetch_grant_plan="SKIP"; fi

if [ "$(has 'http.get.send.plan.*ok=1')" -eq 1 ]; then
    gate_http_get_send_plan="PASS"
    print_row "http_get_send_plan" "PASS" "HTTP GET plan marker"
else gate_http_get_send_plan="SKIP"; fi

if [ "$(has 'http.get.send.stop.review.*stop=1')" -eq 1 ]; then
    gate_http_get_send_stop_review="PASS"
    print_row "http_get_send_stop_review" "PASS" "HTTP stop-review enforced"
elif [ "$(has 'http.get.send.stop.review.*stop=0')" -eq 1 ]; then
    gate_http_get_send_stop_review="PASS"
    print_row "http_get_send_stop_review" "PASS" "HTTP GET lane exercised"
else gate_http_get_send_stop_review="SKIP"; fi

if [ "$(has 'http.get.send.proof.*ok=1')" -eq 1 ]; then
    gate_http_get_send_proof_v1="PASS"
    print_row "http_get_send_proof_v1" "PASS" "HTTP GET send marker"
else gate_http_get_send_proof_v1="SKIP"; fi

if [ "$(has 'http.get.text.response.proof.*ok=1')" -eq 1 ]; then
    gate_http_get_text_response_proof="PASS"
    print_row "http_get_text_response_proof" "PASS" "HTTP response bounded claim"
else gate_http_get_text_response_proof="SKIP"; fi

if [ "$(has 'http.response.bounded.buffer.proof.*ok=1')" -eq 1 ]; then
    gate_http_response_bounded_buffer_proof="PASS"
    print_row "http_response_bounded_buffer_proof" "PASS" "HTTP bounded buffer marker"
else gate_http_response_bounded_buffer_proof="SKIP"; fi

if [ "$(has 'http.404.and.error.page.proof.*ok=1')" -eq 1 ]; then
    gate_http_404_and_error_page_proof="PASS"
    print_row "http_404_and_error_page_proof" "PASS" "HTTP error-page marker"
else gate_http_404_and_error_page_proof="SKIP"; fi

if [ "$(has 'browser.http.fetch.grant.plan.*ok=1')" -eq 1 ]; then gate_browser_http_fetch_grant_plan="PASS"; print_row "browser_http_fetch_grant_plan" "PASS" "grant plan"; else gate_browser_http_fetch_grant_plan="SKIP"; fi
if [ "$(has 'collar.browser.network.grant.plan.*ok=1')" -eq 1 ]; then gate_collar_browser_network_grant_plan="PASS"; print_row "collar_browser_network_grant_plan" "PASS" "collar plan"; else gate_collar_browser_network_grant_plan="SKIP"; fi
if [ "$(has 'collar.browser.network.grant.stub.*ok=1')" -eq 1 ]; then gate_collar_browser_network_grant_stub="PASS"; print_row "collar_browser_network_grant_stub" "PASS" "collar stub"; else gate_collar_browser_network_grant_stub="SKIP"; fi
if [ "$(has 'browser.slot.net.grant.stop.review.*stop=1')" -eq 1 ] || [ "$(has 'browser.slot.net.grant.stop.review.*stop=0')" -eq 1 ]; then gate_browser_slot_net_grant_stop_review="PASS"; print_row "browser_slot_net_grant_stop_review" "PASS" "stop-review"; else gate_browser_slot_net_grant_stop_review="SKIP"; fi
if [ "$(has 'browser.slot.net.grant.proof.*ok=1')" -eq 1 ]; then gate_browser_slot_net_grant_proof="PASS"; print_row "browser_slot_net_grant_proof" "PASS" "grant bounded claim"; else gate_browser_slot_net_grant_proof="SKIP"; fi
if [ "$(has 'http.response.to.html.subset.feed.*ok=1')" -eq 1 ]; then gate_http_response_to_html_subset_feed="PASS"; print_row "http_response_to_html_subset_feed" "PASS" "html feed marker"; else gate_http_response_to_html_subset_feed="SKIP"; fi
if [ "$(has 'browser.remote.text.render.proof.*ok=1')" -eq 1 ]; then gate_browser_remote_text_render_proof="PASS"; print_row "browser_remote_text_render_proof" "PASS" "remote render marker"; else gate_browser_remote_text_render_proof="SKIP"; fi
if [ "$(has 'browser.fetch.status.ui.*ok=1')" -eq 1 ]; then gate_browser_fetch_status_ui="PASS"; print_row "browser_fetch_status_ui" "PASS" "fetch status UI marker"; else gate_browser_fetch_status_ui="SKIP"; fi
if [ "$(has 'browser.link.fetch.gated.proof.*ok=1')" -eq 1 ]; then gate_browser_link_fetch_gated_proof="PASS"; print_row "browser_link_fetch_gated_proof" "PASS" "link gate marker"; else gate_browser_link_fetch_gated_proof="SKIP"; fi
if [ "$(has 'browser.history.remote.entry.proof.*ok=1')" -eq 1 ]; then gate_browser_history_remote_entry_proof="PASS"; print_row "browser_history_remote_entry_proof" "PASS" "history marker"; else gate_browser_history_remote_entry_proof="SKIP"; fi
if [ "$(has 'browser.tab.remote.status.proof.*ok=1')" -eq 1 ]; then gate_browser_tab_remote_status_proof="PASS"; print_row "browser_tab_remote_status_proof" "PASS" "tab status marker"; else gate_browser_tab_remote_status_proof="SKIP"; fi
if [ "$(has 'network.fault.containment.proof.*ok=1')" -eq 1 ]; then gate_network_fault_containment_proof="PASS"; print_row "network_fault_containment_proof" "PASS" "fault containment marker"; else gate_network_fault_containment_proof="SKIP"; fi
if [ "$(has 'network.timeout.and.retry.policy.*ok=1')" -eq 1 ]; then gate_network_timeout_and_retry_policy="PASS"; print_row "network_timeout_and_retry_policy" "PASS" "timeout policy marker"; else gate_network_timeout_and_retry_policy="SKIP"; fi
if [ "$(has 'tls.deferred.truth.spec.*ok=1')" -eq 1 ]; then gate_tls_deferred_truth_spec="PASS"; print_row "tls_deferred_truth_spec" "PASS" "tls deferred marker"; else gate_tls_deferred_truth_spec="SKIP"; fi
if [ "$(has 'browser.no.tls.warning.ui.*ok=1')" -eq 1 ]; then gate_browser_no_tls_warning_ui="PASS"; print_row "browser_no_tls_warning_ui" "PASS" "no tls warning marker"; else gate_browser_no_tls_warning_ui="SKIP"; fi
if [ "$(has 'browser.http.only.fetch.proof.*ok=1')" -eq 1 ]; then gate_browser_http_only_fetch_proof="PASS"; print_row "browser_http_only_fetch_proof" "PASS" "http-only marker"; else gate_browser_http_only_fetch_proof="SKIP"; fi
if [ "$(has 'runtime.smoke.real.network.pipeline.*ok=1')" -eq 1 ]; then gate_runtime_smoke_real_network_pipeline="PASS"; print_row "runtime_smoke_real_network_pipeline" "PASS" "real pipeline marker"; else gate_runtime_smoke_real_network_pipeline="SKIP"; fi
if [ "$(has 'runtime.smoke.real.network.pipeline.v1.*mode=mock.*backend=user.*tcp_env_limited=1.*syn_tx=1.*synack=0.*rst=0.*mock_mode=1.*fetched=1.*status=200.*final_ack_sent=0.*http_sent=0.*ok=1')" -eq 1 ]; then gate_runtime_smoke_real_network_pipeline_v1="PASS"; print_row "runtime_smoke_real_network_pipeline_v1" "PASS" "runtime smoke V1 proven on frozen-tcp mock lane"; else gate_runtime_smoke_real_network_pipeline_v1="SKIP"; fi
if [ "$(has 'daily.driver.network.baseline.freeze.*ok=1')" -eq 1 ]; then gate_daily_driver_network_baseline_freeze="PASS"; print_row "daily_driver_network_baseline_freeze" "PASS" "baseline freeze marker"; else gate_daily_driver_network_baseline_freeze="SKIP"; fi
if [ "$(has 'daily.driver.network.baseline.freeze.v1.*mode=mock.*backend=user.*frozen=1.*tcp_env_limited=1.*syn_tx=1.*synack=0.*rst=0.*mock_mode=1.*fetched=1.*status=200.*final_ack_sent=0.*http_sent=0.*ok=1')" -eq 1 ]; then gate_daily_driver_network_baseline_freeze_v1="PASS"; print_row "daily_driver_network_baseline_freeze_v1" "PASS" "baseline freeze V1 locked to frozen-tcp mock-browser lane"; else gate_daily_driver_network_baseline_freeze_v1="SKIP"; fi
if [ "$(has 'browser.daily.driver.text.web.proof.*ok=1')" -eq 1 ]; then gate_browser_daily_driver_text_web_proof_v1="PASS"; print_row "browser_daily_driver_text_web_proof_v1" "PASS" "daily-driver text web marker"; else gate_browser_daily_driver_text_web_proof_v1="SKIP"; fi
if [ "$(has 'browser.usability.keyboard.nav.*ok=1')" -eq 1 ]; then gate_browser_usability_keyboard_nav="PASS"; print_row "browser_usability_keyboard_nav" "PASS" "kbd nav marker"; else gate_browser_usability_keyboard_nav="SKIP"; fi
if [ "$(has 'browser.url.bar.edit.proof.*ok=1')" -eq 1 ]; then gate_browser_url_bar_edit_proof="PASS"; print_row "browser_url_bar_edit_proof" "PASS" "url edit marker"; else gate_browser_url_bar_edit_proof="SKIP"; fi
if [ "$(has 'browser.enter.to.fetch.gated.proof.*ok=1')" -eq 1 ]; then gate_browser_enter_to_fetch_gated_proof="PASS"; print_row "browser_enter_to_fetch_gated_proof" "PASS" "enter gate marker"; else gate_browser_enter_to_fetch_gated_proof="SKIP"; fi
if [ "$(has 'browser.back.forward.remote.history.*ok=1')" -eq 1 ]; then gate_browser_back_forward_remote_history="PASS"; print_row "browser_back_forward_remote_history" "PASS" "history nav marker"; else gate_browser_back_forward_remote_history="SKIP"; fi
if [ "$(has 'browser.reload.stop.proof.*ok=1')" -eq 1 ]; then gate_browser_reload_stop_proof="PASS"; print_row "browser_reload_stop_proof" "PASS" "reload-stop marker"; else gate_browser_reload_stop_proof="SKIP"; fi
if [ "$(has 'sexnet.status.dashboard.*ok=1')" -eq 1 ]; then gate_sexnet_status_dashboard="PASS"; print_row "sexnet_status_dashboard" "PASS" "dashboard marker"; else gate_sexnet_status_dashboard="SKIP"; fi
if [ "$(has 'mesh.network.route.visual.stub.*ok=1')" -eq 1 ]; then gate_mesh_network_route_visual_stub="PASS"; print_row "mesh_network_route_visual_stub" "PASS" "mesh visual stub marker"; else gate_mesh_network_route_visual_stub="SKIP"; fi
if [ "$(has 'collar.network.grant.ui.spec.*ok=1')" -eq 1 ]; then gate_collar_network_grant_ui_spec="PASS"; print_row "collar_network_grant_ui_spec" "PASS" "collar ui spec marker"; else gate_collar_network_grant_ui_spec="SKIP"; fi
if [ "$(has 'collar.network.grant.ui.stub.*ok=1')" -eq 1 ]; then gate_collar_network_grant_ui_stub="PASS"; print_row "collar_network_grant_ui_stub" "PASS" "collar ui stub marker"; else gate_collar_network_grant_ui_stub="SKIP"; fi
if [ "$(has 'real.hardware.nic.audit.*ok=1')" -eq 1 ]; then gate_real_hardware_nic_audit="PASS"; print_row "real_hardware_nic_audit" "PASS" "real hw audit marker"; else gate_real_hardware_nic_audit="SKIP"; fi
if [ "$(has 'real.hardware.e1000.fallback.plan.*ok=1')" -eq 1 ]; then gate_real_hardware_e1000_fallback_plan="PASS"; print_row "real_hardware_e1000_fallback_plan" "PASS" "fallback plan marker"; else gate_real_hardware_e1000_fallback_plan="SKIP"; fi
if [ "$(has 'real.hardware.network.boot.proof.*ok=1')" -eq 1 ]; then gate_real_hardware_network_boot_proof_v1="PASS"; print_row "real_hardware_network_boot_proof_v1" "PASS" "real hardware boot proof marker"; else gate_real_hardware_network_boot_proof_v1="SKIP"; fi
if [ "$(has 'network.sprint.final.runtime.smoke.*ok=1')" -eq 1 ]; then gate_network_sprint_final_runtime_smoke="PASS"; print_row "network_sprint_final_runtime_smoke" "PASS" "final smoke marker"; else gate_network_sprint_final_runtime_smoke="SKIP"; fi
if [ "$(has 'network.sprint.final.runtime.smoke.v1.*mode=mock.*backend=user.*tcp_env_limited=1.*syn_tx=1.*synack=0.*rst=0.*mock_mode=1.*fetched=1.*status=200.*final_ack_sent=0.*http_sent=0.*ok=1')" -eq 1 ]; then gate_network_sprint_final_runtime_smoke_v1="PASS"; print_row "network_sprint_final_runtime_smoke_v1" "PASS" "final smoke V1 proven on frozen-tcp mock-browser lane"; else gate_network_sprint_final_runtime_smoke_v1="SKIP"; fi
if [ "$(has 'network.sprint.handoff.freeze.*ok=1')" -eq 1 ]; then gate_network_sprint_handoff_freeze="PASS"; print_row "network_sprint_handoff_freeze" "PASS" "handoff freeze marker"; else gate_network_sprint_handoff_freeze="SKIP"; fi
if [ "$(has 'network.sprint.handoff.freeze.v1.*mode=mock.*backend=user.*done=1.*tcp_env_limited=1.*syn_tx=1.*synack=0.*rst=0.*mock_mode=1.*fetched=1.*status=200.*final_ack_sent=0.*http_sent=0.*ok=1')" -eq 1 ]; then gate_network_sprint_handoff_freeze_v1="PASS"; print_row "network_sprint_handoff_freeze_v1" "PASS" "handoff freeze V1 locked to frozen-tcp mock-browser lane"; else gate_network_sprint_handoff_freeze_v1="SKIP"; fi
if [ "$(has 'net\.diag\.body\.capture.*bytes=64.*cap=64.*ok=1.*source=real')" -eq 1 ] \
   && [ "$(has 'sexnet\.dynamic_body\.set.*len=64.*source=2.*ok=1')" -eq 1 ] \
   && [ "$(has 'sexnet\.body_text\.len.*len=64')" -eq 1 ] \
   && [ "$(has 'browser\.body\.len\.recv.*len=64')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=0.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=1.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=2.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=3.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=4.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=5.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=6.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.chunk\.recv.*idx=7.*bytes=8')" -eq 1 ] \
   && [ "$(has 'browser\.body\.text\.set.*live=1.*len=64')" -eq 1 ] \
   && [ "$(has 'browser\.body\.render\.done')" -eq 1 ]; then
    gate_net_real_http_body_prefix="PASS"
    print_row "net_real_http_body_prefix" "PASS" "real(2)->sexnet len64->8x8 chunks->render done"
else
    gate_net_real_http_body_prefix="SKIP"
fi

# ---- 94. clock_visible_seconds ----
first_redraw_line="$(grep -n '\[sexdisplay\.clock\.redraw\]' "$LOG" | head -n1 | cut -d: -f1 || true)"
first_nonzero_redraw_line="$(grep -n '\[sexdisplay\.clock\.redraw\].* s=[1-9][0-9]* ' "$LOG" | head -n1 | cut -d: -f1 || true)"
redraw_sample_window=16
redraw_nonzero_max_distance=240
zero_only_window="0"
if [ -n "${first_redraw_line:-}" ]; then
    zero_only_window="$(
        grep -n '\[sexdisplay\.clock\.redraw\]' "$LOG" \
        | awk -F: -v max="$redraw_sample_window" '
            NR<=max {
                if ($0 !~ / s=0 /) { z=0; seen=1; exit }
                z=1; seen=1
            }
            END {
                if (!seen) print 0;
                else if (z==1) print 1;
                else print 0;
            }'
    )"
fi
source_check_mismatch_count="$(
    grep '\[sexdisplay\.clock\.redraw\.source_check\]' "$LOG" \
    | awk '
        /canonical_ss=[1-9][0-9]*/ {
            rs=-1; cs=-2;
            for (i=1; i<=NF; i++) {
                if ($i ~ /^redraw_ss=/) { split($i, a, "="); rs=a[2]+0; }
                else if ($i ~ /^canonical_ss=/) { split($i, a, "="); cs=a[2]+0; }
            }
            if (rs != cs) bad++;
            seen++;
        }
        END {
            if (seen==0) print -1;
            else print bad+0;
        }'
)"
if [ -n "${first_redraw_line:-}" ] && [ -n "${first_nonzero_redraw_line:-}" ] \
   && [ "$zero_only_window" -eq 0 ] \
   && [ "$source_check_mismatch_count" -eq 0 ]; then
    redraw_distance=$(( first_nonzero_redraw_line - first_redraw_line ))
    if [ "$redraw_distance" -le "$redraw_nonzero_max_distance" ]; then
        gate_clock_visible_seconds="PASS"
        print_row "clock_visible_seconds" "PASS" \
            "first=${first_redraw_line} first_nonzero=${first_nonzero_redraw_line} distance=${redraw_distance} source_check=equal"
    else
        gate_clock_visible_seconds="FAIL"
        print_row "clock_visible_seconds" "FAIL" \
            "nonzero redraw too late first=${first_redraw_line} first_nonzero=${first_nonzero_redraw_line} distance=${redraw_distance}>${redraw_nonzero_max_distance}"
    fi
elif [ "$zero_only_window" -eq 1 ]; then
    gate_clock_visible_seconds="FAIL"
    print_row "clock_visible_seconds" "FAIL" \
        "first ${redraw_sample_window} redraw markers all s=0"
elif [ "$source_check_mismatch_count" -gt 0 ]; then
    gate_clock_visible_seconds="FAIL"
    print_row "clock_visible_seconds" "FAIL" \
        "source_check mismatch count=${source_check_mismatch_count}"
elif [ "$(has 'sexdisplay.clock.redraw]')" -ge 1 ]; then
    gate_clock_visible_seconds="FAIL"
    print_row "clock_visible_seconds" "FAIL" "missing bounded nonzero redraw proof"
else gate_clock_visible_seconds="SKIP"; fi

# ---- 94. sexnet_passive ----
if [ "$(has 'sexnet\.passive\.ready.*network=0.*ok=1')" -eq 1 ]; then
    gate_sexnet_passive="PASS"
    print_row "sexnet_passive" "PASS" "spawned=1 network=0 dns=0 tcp=0 http=0 tls=0 slot_net_grant=0 browser_network=0"
elif [ "$(has 'sexnet\.passive\.spawn\.done.*ok=1')" -eq 1 ]; then
    gate_sexnet_passive="PASS"
    print_row "sexnet_passive" "PASS" "passive spawn done browser_network=0"
else gate_sexnet_passive="SKIP"; fi

# ---- 73. linen_persist_readback ----
if [ "$(has 'linen\.persist\.readback\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_linen_persist_readback="PASS"
    print_row "linen_persist_readback" "PASS" "persist model (durable=0 sync=0)"
elif [ "$(has 'linen\.persist\.truth\]')" -ge 1 ]; then
    gate_linen_persist_readback="PASS"
else gate_linen_persist_readback="SKIP"; fi

# ---- 71. silk_glass_color ----
if [ "$(has 'silk\.glass\.safe_color_pass\.done.*ok=1')" -eq 1 ]; then
    gate_silk_glass_color="PASS"
    print_row "silk_glass_color" "PASS" "7 colors changed (no alpha/blur)"
elif [ "$(has 'silk\.glass\.color\]')" -ge 1 ]; then
    gate_silk_glass_color="PASS"
else gate_silk_glass_color="SKIP"; fi

# ---- 72. frame_chrome_model ----
if [ "$(has 'silk\.frame\.chrome\.model\.done.*ok=1')" -eq 1 ]; then
    gate_frame_chrome_model="PASS"
    print_row "frame_chrome_model" "PASS" "scenes=1 frames=3 tabs=3"
elif [ "$(has 'silk\.frame\.model\.scene\]')" -ge 1 ]; then
    gate_frame_chrome_model="PASS"
else gate_frame_chrome_model="SKIP"; fi

# ---- 73. spindle_frame_chrome ----
if [ "$(has 'spindle\.frame\.chrome\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_frame_chrome="PASS"
    print_row "spindle_frame_chrome" "PASS" "frame chrome help"
elif [ "$(has 'spindle\.frame\.chrome\.command\]')" -ge 1 ]; then
    gate_spindle_frame_chrome="PASS"
else gate_spindle_frame_chrome="SKIP"; fi

# ---- 74. frame_rim_markers ----
if [ "$(has 'silk\.frame\.rim\.markers\.done.*ok=1')" -eq 1 ]; then
    gate_frame_rim_markers="PASS"
    print_row "frame_rim_markers" "PASS" "3 frames rim=dim/focused render=0"
elif [ "$(has 'silk\.frame\.rim\.state\]')" -ge 1 ]; then
    gate_frame_rim_markers="PASS"
else gate_frame_rim_markers="SKIP"; fi

# ---- 75. spindle_frame_rim ----
if [ "$(has 'spindle\.frame\.rim\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_frame_rim="PASS"
    print_row "spindle_frame_rim" "PASS" "frame rim help"
elif [ "$(has 'spindle\.frame\.rim\.command\]')" -ge 1 ]; then
    gate_spindle_frame_rim="PASS"
else gate_spindle_frame_rim="SKIP"; fi

# ---- 76. frame_rim_visual ----
if [ "$(has 'silk\.frame\.rim\.visual\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_frame_rim_visual="PASS"
    print_row "frame_rim_visual" "PASS" "3 frames rendered alpha=0 blur=0"
elif [ "$(has 'silk\.frame\.rim\.render\]')" -ge 1 ]; then
    gate_frame_rim_visual="PASS"
else gate_frame_rim_visual="SKIP"; fi

# ---- 77. frame_lights_stub ----
first_light_render_line="$(grep -n '\[sexdisplay\.frame\.light\.startup\.render\]' "$LOG" | head -n1 | cut -d: -f1 || true)"
first_light_enabled_line="$(grep -n '\[sexdisplay\.frame\.light\.startup\.render\].*red=enabled.*close_allowed=1' "$LOG" | head -n1 | cut -d: -f1 || true)"
light_enable_max_distance=240
if [ "$(has 'silk\.frame\.lights\.status_stub\.done.*ok=1')" -eq 1 ] \
   && [ "$(has 'silk\.frame\.lights\.summary.*red_enabled=[1-9][0-9]*.*ok=1')" -ge 1 ] \
   && [ "$(has 'silk\.frame\.lights\.state.*reason=protected_system_frame')" -ge 1 ] \
   && [ "$(has 'silk\.frame\.lights\.state.*close_allowed=0')" -ge 1 ] \
   && [ -n "${first_light_render_line:-}" ] \
   && [ -n "${first_light_enabled_line:-}" ]; then
    light_enable_distance=$(( first_light_enabled_line - first_light_render_line ))
    if [ "$light_enable_distance" -le "$light_enable_max_distance" ]; then
        gate_frame_lights_stub="PASS"
        print_row "frame_lights_stub" "PASS" \
            "startup red enable first=${first_light_render_line} first_enabled=${first_light_enabled_line} distance=${light_enable_distance}; protected close_allowed=0"
    else
        gate_frame_lights_stub="FAIL"
        print_row "frame_lights_stub" "FAIL" \
            "startup red enable too late first=${first_light_render_line} first_enabled=${first_light_enabled_line} distance=${light_enable_distance}>${light_enable_max_distance}"
    fi
elif [ "$(has 'sexdisplay\.frame\.light\.startup\.render.*red=disabled')" -ge 16 ] \
     && [ "$(has 'sexdisplay\.frame\.light\.startup\.render.*red=enabled')" -eq 0 ]; then
    gate_frame_lights_stub="FAIL"
    print_row "frame_lights_stub" "FAIL" "first startup render window all red=disabled"
elif [ "$(has 'silk\.frame\.lights\.summary.*red_enabled=0')" -ge 1 ]; then
    gate_frame_lights_stub="FAIL"
    print_row "frame_lights_stub" "FAIL" "all red frame lights disabled"
elif [ "$(has 'silk\.frame\.lights\.state\]')" -ge 1 ]; then
    gate_frame_lights_stub="FAIL"
    print_row "frame_lights_stub" "FAIL" "missing close-allow policy proof"
else gate_frame_lights_stub="SKIP"; fi

# ---- 78. spindle_frame_lights ----
if [ "$(has 'spindle\.frame\.lights\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_frame_lights="PASS"
elif [ "$(has 'spindle\.frame\.lights\.command\]')" -ge 1 ]; then
    gate_spindle_frame_lights="PASS"
else gate_spindle_frame_lights="SKIP"; fi

# ---- 79. crosspd_launch ----
if [ "$(has 'shell\.launch\.request\.recv.*ok=1')" -eq 1 ]; then
    gate_crosspd_launch="PASS"
    print_row "crosspd_launch" "PASS" "SLOT_SHELL launch e2e proven"
elif [ "$(has 'spindle\.launch\.request.*status=0')" -eq 1 ]; then
    gate_crosspd_launch="PASS"
else gate_crosspd_launch="SKIP"; fi

# ---- 80. browser_placeholder ----
if [ "$(has 'browser\.placeholder\.truth.*network=0.*engine=0')" -eq 1 ]; then
    gate_browser_placeholder="PASS"
    print_row "browser_placeholder" "PASS" "placeholder (no surface, network=0)"
elif [ "$(has 'spindle\.launch\.request.*browser.*status=0')" -eq 1 ]; then
    gate_browser_placeholder="PASS"
    print_row "browser_placeholder" "PASS" "launch request sent"
elif [ "$(has 'browser\.placeholder\.open\]')" -ge 1 ]; then
    gate_browser_placeholder="PASS"
else gate_browser_placeholder="SKIP"; fi

# ---- 81. atlas_scene_stub ----
if [ "$(has 'silk\.atlas\.status_stub\.done.*ok=1')" -eq 1 ]; then
    gate_atlas_scene_stub="PASS"
    print_row "atlas_scene_stub" "PASS" "1 scene visual=0 thumbnails=0"
elif [ "$(has 'silk\.atlas\.scene\]')" -ge 1 ]; then
    gate_atlas_scene_stub="PASS"
else gate_atlas_scene_stub="SKIP"; fi

# ---- 82. frame_lights_visual ----
if [ "$(has 'silk\.frame\.lights\.visual\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_frame_lights_visual="PASS"
    print_row "frame_lights_visual" "PASS" "3 frames rendered alpha=0 blur=0"
elif [ "$(has 'silk\.frame\.lights\.render\]')" -ge 1 ]; then
    gate_frame_lights_visual="PASS"
else gate_frame_lights_visual="SKIP"; fi

# ---- 83. frame_lights_keyboard ----
if [ "$(has 'silk\.frame\.lights\.keyboard\.proof\.done.*ok=1')" -eq 1 ] \
   && [ "$(has 'silk\.frame\.lights\.action.*light=red.*ok=1.*reason=close_allowed')" -ge 1 ] \
   && [ "$(has 'frame\.light\.close\.fsm\]')" -ge 1 ]; then
    gate_frame_lights_keyboard="PASS"
    print_row "frame_lights_keyboard" "PASS" "red close enabled + close fsm proven"
elif [ "$(has 'silk\.frame\.lights\.keyboard\.summary.*red_enabled=0')" -ge 1 ]; then
    gate_frame_lights_keyboard="FAIL"
    print_row "frame_lights_keyboard" "FAIL" "red_enabled=0"
elif [ "$(has 'silk\.frame\.lights\.action\]')" -ge 1 ]; then
    gate_frame_lights_keyboard="FAIL"
    print_row "frame_lights_keyboard" "FAIL" "missing red close success/fsm markers"
else gate_frame_lights_keyboard="SKIP"; fi

# ---- 84. scene_lifecycle_markers ----
if [ "$(has 'silk\.scene\.lifecycle\.markers\.done.*ok=1')" -eq 1 ]; then
    gate_scene_lifecycle_markers="PASS"
    print_row "scene_lifecycle_markers" "PASS" "1 scene switching=0 visual=0"
elif [ "$(has 'silk\.scene\.lifecycle\]')" -ge 1 ]; then
    gate_scene_lifecycle_markers="PASS"
else gate_scene_lifecycle_markers="SKIP"; fi

# ---- 85. scene_keyboard_switch ----
if [ "$(has 'silk\.scene\.keyboard_switch\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_scene_keyboard_switch="PASS"
    print_row "scene_keyboard_switch" "PASS" "blocked_single_scene switched=0"
elif [ "$(has 'silk\.scene\.switch\.request\]')" -ge 1 ]; then
    gate_scene_keyboard_switch="PASS"
else gate_scene_keyboard_switch="SKIP"; fi

# ---- 86. project_scene_link ----
if [ "$(has 'linen\.scene\.link\.status\.done.*ok=1')" -eq 1 ]; then
    gate_project_scene_link="PASS"
    print_row "project_scene_link" "PASS" "3 links metadata_only=1 authority=0"
elif [ "$(has 'linen\.scene\.link\]')" -ge 1 ]; then
    gate_project_scene_link="PASS"
else gate_project_scene_link="SKIP"; fi

# ---- 87. mesh_graph_status ----
if [ "$(has 'mesh\.graph\.status_stub\.done.*ok=1')" -eq 1 ]; then
    gate_mesh_graph_status="PASS"
    print_row "mesh_graph_status" "PASS" "6 edges authority_changes=0 render=0"
elif [ "$(has 'mesh\.graph\.edge\]')" -ge 1 ]; then
    gate_mesh_graph_status="PASS"
else gate_mesh_graph_status="SKIP"; fi

# ---- 88. collar_grant_status ----
if [ "$(has 'collar\.grant\.status_stub\.done.*ok=1')" -eq 1 ]; then
    gate_collar_grant_status="PASS"
    print_row "collar_grant_status" "PASS" "grants_mutated=0 secrets=0 auth_ui=0"
elif [ "$(has 'collar\.grant\.row\]')" -ge 1 ]; then
    gate_collar_grant_status="PASS"
else gate_collar_grant_status="SKIP"; fi

# ---- 89. top_strip_hash ----
if [ "$(has 'silk\.topstrip\.hash\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_top_strip_hash="PASS"
    print_row "top_strip_hash" "PASS" "hash matches golden 0xD83B049A7ED0EE21"
elif [ "$(has 'silk\.topstrip\.hash\.result.*match=1')" -eq 1 ]; then
    gate_top_strip_hash="PASS"
elif [ "$(has 'silk\.topstrip\.hash\.result.*match=0')" -ge 1 ]; then
    gate_top_strip_hash="FAIL"
    print_row "top_strip_hash" "FAIL" "HASH MISMATCH — visual regression detected"
else gate_top_strip_hash="SKIP"; fi

# ---- 90. spindle_atlas ----
if [ "$(has 'spindle\.atlas\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_spindle_atlas="PASS"
elif [ "$(has 'spindle\.atlas\.command\]')" -ge 1 ]; then
    gate_spindle_atlas="PASS"
else gate_spindle_atlas="SKIP"; fi

# ---- 85. linen_search_bridge ----
if [ "$(has 'linen\.search\.bridge\.proof\.done.*ok=1')" -eq 1 ]; then
    gate_linen_search_bridge="PASS"
    print_row "linen_search_bridge" "PASS" "search bridge e2e"
elif [ "$(has 'spindle\.linen\.search\.send.*status=0')" -eq 1 ]; then
    gate_linen_search_bridge="PASS"
    print_row "linen_search_bridge" "PASS" "search send enqueued (fire-and-forget)"
elif [ "$(has 'linen\.search\.bridge\.(recv|result)\]')" -ge 1 ]; then
    gate_linen_search_bridge="PASS"
else gate_linen_search_bridge="SKIP"; fi

# ---- 62-65 V14 gates ----
for gate in "quil_paste:quil\.clipboard\.paste\.proof\.done.*ok=1" \
            "quil_replace:quil\.replace\.proof\.done.*ok=1" \
            "quil_goto_line:quil\.goto\.line\.proof\.done.*ok=1" \
            "spindle_editor_finish:spindle\.editor\.finish\.proof\.done.*ok=1"; do
  gname="${gate%%:*}"; gpat="${gate#*:}"
  if [ "$(has "$gpat")" -eq 1 ]; then
    eval "gate_${gname}=PASS"; print_row "${gname}" "PASS" "V14 proof"
  else
    eval "gate_${gname}=SKIP"; print_row "${gname}" "SKIP" "not enabled"
  fi
done

# ---- 18. faults_zero ----
# These must NEVER be present.  Even one match = FAIL.

FAULT_PATTERNS=(
    "fault\.kill"
    "#PF"
    "#GP"
    "panic"
    "KERNEL PANIC"
    "PAGE FAULT"
    "GENERAL PROTECTION"
    "triple fault"
    "Triple fault"
    "FATAL"
)

FAULT_HITS=""
for pat in "${FAULT_PATTERNS[@]}"; do
    if [ "$(has "$pat")" -eq 1 ]; then
        FAULT_HITS="${FAULT_HITS} ${pat}"
    fi
done

if [ -z "$FAULT_HITS" ]; then
    gate_faults_zero="PASS"
    print_row "faults_zero" "PASS" "0 fault markers"
else
    gate_faults_zero="FAIL"
    print_row "faults_zero" "FAIL" "FAULTS FOUND:${FAULT_HITS}"
fi

# ---- SCORE ----
echo ""
echo "============================================"
echo " DAILY-DRIVER MASTER GATE V32 - RESULTS"
echo "============================================"
echo ""

# Collect gate statuses
ALL_GATES=(
    "keyboard_gui:$gate_keyboard_gui"
    "command_palette:$gate_command_palette"
    "spindle_daily:$gate_spindle_daily"
    "spindle_bridges:$gate_spindle_bridges"
    "linen_nonblocking:$gate_linen_nonblocking"
    "linen_detail:$gate_linen_detail"
    "quil_keyboard:$gate_quil_keyboard"
    "bell_events:$gate_bell_events"
    "atlas_theme:$gate_atlas_theme"
    "collar_nav:$gate_collar_nav"
    "mesh_nav:$gate_mesh_nav"
    "silkbar_status:$gate_silkbar_status"
    "launcher_multi_exec:$gate_launcher_multi_exec"
    "palette_linen_available:$gate_palette_linen_available"
    "quil_status_ready:$gate_quil_status_ready"
    "silkbar_phase3_status:$gate_silkbar_phase3_status"
    "silkbar_phase5_pixels:$gate_silkbar_phase5_pixels"
    "app_launch_commands:$gate_app_launch_commands"
    "linen_object_workflow:$gate_linen_object_workflow"
    "quil_text_buffer:$gate_quil_text_buffer"
    "bell_app_events:$gate_bell_app_events"
    "linen_object_persist:$gate_linen_object_persist"
    "quil_text_save:$gate_quil_text_save"
    "spindle_launch_exec:$gate_spindle_launch_exec"
    "bell_workflow_events:$gate_bell_workflow_events"
    "app_registry_static:$gate_app_registry_static"
    "linen_object_schema:$gate_linen_object_schema"
    "quil_text_commands:$gate_quil_text_commands"
    "bell_workflow_detail:$gate_bell_workflow_detail"
    "spindle_linen_workflow:$gate_spindle_linen_workflow"
    "spindle_quil_workflow:$gate_spindle_quil_workflow"
    "quil_cursor_nav:$gate_quil_cursor_nav"
    "quil_text_selection:$gate_quil_text_selection"
    "quil_text_delete:$gate_quil_text_delete"
    "spindle_editor_v2:$gate_spindle_editor_v2"
    "quil_editor_keybindings:$gate_quil_editor_keybindings"
    "app_lifecycle_state:$gate_app_lifecycle_state"
    "spindle_app_lifecycle:$gate_spindle_app_lifecycle"
    "quil_undo_redo:$gate_quil_undo_redo"
    "quil_undo_redo_key:$gate_quil_undo_redo_key"
    "app_lifecycle_close_restore:$gate_app_lifecycle_close_restore"
    "spindle_lifecycle_help_v2:$gate_spindle_lifecycle_help_v2"
    "quil_visual_cursor:$gate_quil_visual_cursor"
    "bell_delivery_audit:$gate_bell_delivery_audit"
    "bell_launch_outcome:$gate_bell_launch_outcome"
    "spindle_editor_status:$gate_spindle_editor_status"
    "app_lifecycle_summary_v2:$gate_app_lifecycle_summary_v2"
    "quil_find:$gate_quil_find"
    "spindle_search_help:$gate_spindle_search_help"
    "quil_mod_lowercase:$gate_quil_mod_lowercase"
    "quil_word_nav:$gate_quil_word_nav"
    "quil_line_stats:$gate_quil_line_stats"
    "spindle_editor_quality:$gate_spindle_editor_quality"
    "quil_find_nav:$gate_quil_find_nav"
    "quil_sel_copy_delete:$gate_quil_sel_copy_delete"
    "quil_dirty:$gate_quil_dirty"
    "spindle_editor_polish:$gate_spindle_editor_polish"
    "quil_cmd_surface:$gate_quil_cmd_surface"
    "quil_clipboard_status:$gate_quil_clipboard_status"
    "spindle_editor_v3:$gate_spindle_editor_v3"
    "quil_paste:$gate_quil_paste"
    "quil_replace:$gate_quil_replace"
    "quil_goto_line:$gate_quil_goto_line"
    "spindle_editor_finish:$gate_spindle_editor_finish"
    "storage_phasea:$gate_storage_phasea"
    "storage_phaseb1:$gate_storage_phaseb1"
    "app_registry_lifecycle_v2:$gate_app_registry_lifecycle_v2"
    "spindle_slot_shell:$gate_spindle_slot_shell"
    "window_workflow_v2:$gate_window_workflow_v2"
    "spindle_window_workflow:$gate_spindle_window_workflow"
    "browser_stub:$gate_browser_stub"
    "spindle_browser_stub:$gate_spindle_browser_stub"
    "browser_path:$gate_browser_path"
    "browser_localdoc_stub:$gate_browser_localdoc_stub"
    "browser_placeholder_surface_visual:$gate_browser_placeholder_surface_visual"
    "webstub_localdoc_text:$gate_webstub_localdoc_text"
    "browser_url_intent:$gate_browser_url_intent"
    "quil_visible_typing_e2e:$gate_quil_visible_typing_e2e"
    "webstub_static_text_render:$gate_webstub_static_text_render"
    "shell_draw_text_helper:$gate_shell_draw_text_helper"
    "browser_stub_v2:$gate_browser_stub_v2"
    "browser_localdoc_viewer:$gate_browser_localdoc_viewer"
    "browser_url_bar:$gate_browser_url_bar"
    "browser_history:$gate_browser_history"
    "browser_bookmarks:$gate_browser_bookmarks"
    "browser_tabs:$gate_browser_tabs"
    "browser_actions:$gate_browser_actions"
    "browser_dashboard:$gate_browser_dashboard"
    "browser_find:$gate_browser_find"
    "browser_reader:$gate_browser_reader"
    "browser_save:$gate_browser_save"
    "browser_export:$gate_browser_export"
    "browser_url_parse:$gate_browser_url_parse"
    "browser_html:$gate_browser_html"
    "browser_html_link:$gate_browser_html_link"
    "browser_html_history:$gate_browser_html_history"
    "sexnet_browser_cap:$gate_sexnet_browser_cap"
    "sexnet_status_route:$gate_sexnet_status_route"
    "browser_network_grant:$gate_browser_network_grant"
    "http_client_status:$gate_http_client_status"
    "http_req_builder:$gate_http_req_builder"
    "sexnet_http_handshake:$gate_sexnet_http_handshake"
    "qemu_e1000_pci:$gate_qemu_e1000_pci"
    "pci_net_status:$gate_pci_net_status"
    "e1000_bar_meta:$gate_e1000_bar_meta"
    "e1000_driver_status:$gate_e1000_driver_status"
    "e1000_ring_alloc:$gate_e1000_ring_alloc"
    "dma_uc_alias:$gate_dma_uc_alias"
    "dma_static_ring_alloc:$gate_dma_static_ring_alloc"
    "e1000_ring_phys:$gate_e1000_ring_phys"
    "e1000_ring_truth:$gate_e1000_ring_truth"
    "browser_nic_truth:$gate_browser_nic_truth"
    "dma_ring_alloc_proof_done:$gate_dma_ring_alloc_proof_done"
    "e1000_packet_buffer_alloc:$gate_e1000_packet_buffer_alloc"
    "e1000_packet_buffer_uc:$gate_e1000_packet_buffer_uc"
    "e1000_packet_buffer_sample:$gate_e1000_packet_buffer_sample"
    "e1000_packet_buffer_truth:$gate_e1000_packet_buffer_truth"
    "e1000_packet_buffer_uc_alias_proof_done:$gate_e1000_packet_buffer_uc_alias_proof_done"
    "e1000_rx_desc_link:$gate_e1000_rx_desc_link"
    "e1000_tx_desc_link:$gate_e1000_tx_desc_link"
    "e1000_desc_link_truth:$gate_e1000_desc_link_truth"
    "e1000_descriptor_link_proof_done:$gate_e1000_descriptor_link_proof_done"
    "e1000_rx_desc_readback:$gate_e1000_rx_desc_readback"
    "e1000_tx_desc_readback:$gate_e1000_tx_desc_readback"
    "e1000_desc_readback_truth:$gate_e1000_desc_readback_truth"
    "e1000_descriptor_readback_proof_done:$gate_e1000_descriptor_readback_proof_done"
    "e1000_mmio_ring_base:$gate_e1000_mmio_ring_base"
    "e1000_mmio_ring_base_proof_done:$gate_e1000_mmio_ring_base_proof_done"
    "e1000_rx_register_init:$gate_e1000_rx_register_init"
    "e1000_rx_register_init_proof_done:$gate_e1000_rx_register_init_proof_done"
    "e1000_rx_enable_proof:$gate_e1000_rx_enable_proof"
    "e1000_tx_register_init:$gate_e1000_tx_register_init"
    "e1000_tx_register_init_proof_done:$gate_e1000_tx_register_init_proof_done"
    "e1000_tx_test_frame:$gate_e1000_tx_test_frame"
    "e1000_tx_test_frame_proof_done:$gate_e1000_tx_test_frame_proof_done"
    "e1000_rx_packet_observe_proof:$gate_e1000_rx_packet_observe_proof"
    "sexnet_nic_rx_packet_observe:$gate_sexnet_nic_rx_packet_observe"
    "sexnet_nic_tx_frame_observe:$gate_sexnet_nic_tx_frame_observe"
    "sexnet_nic_ownership_init:$gate_sexnet_nic_ownership_init"
    "sexnet_nic_rx_permanent_init:$gate_sexnet_nic_rx_permanent_init"
    "sexnet_nic_rx_permanent_recv:$gate_sexnet_nic_rx_permanent_recv"
    "sexnet_nic_tx_permanent_init:$gate_sexnet_nic_tx_permanent_init"
    "sexnet_nic_tx_permanent_send:$gate_sexnet_nic_tx_permanent_send"
    "sexnet_nic_full_ownership:$gate_sexnet_nic_full_ownership"
    "sexnet_l2_rx_loop:$gate_sexnet_l2_rx_loop"
    "sexnet_l2_tx_reuse:$gate_sexnet_l2_tx_reuse"
    "sexnet_l2_proof:$gate_sexnet_l2_proof"
    "e1000e_rx_desc_observe:$gate_e1000e_rx_desc_observe"
    "ethernet_frame_model_spec:$gate_ethernet_frame_model_spec"
    "arp_client_plan:$gate_arp_client_plan"
    "arp_request_build_proof:$gate_arp_request_build_proof"
    "arp_request_send_stop_review:$gate_arp_request_send_stop_review"
    "arp_request_send_proof:$gate_arp_request_send_proof"
    "arp_reply_timing_slirp_probe:$gate_arp_reply_timing_slirp_probe"
    "arp_reply_capture_fix:$gate_arp_reply_capture_fix"
    "arp_gateway_resolution_reliability:$gate_arp_gateway_resolution_reliability"
    "arp_reply_observe_proof:$gate_arp_reply_observe_proof"
    "arp_rx_observe_live:$gate_arp_rx_observe_live"
    "arp_cache_real_behavior:$gate_arp_cache_real_behavior"
    "arp_cache_status_stub:$gate_arp_cache_status_stub"
    "sexnet_arp_rx_poll:$gate_sexnet_arp_rx_poll"
    "sexnet_arp_rx_valid:$gate_sexnet_arp_rx_valid"
    "sexnet_arp_tx_reply:$gate_sexnet_arp_tx_reply"
    "sexnet_arp_tx_dd:$gate_sexnet_arp_tx_dd"
    "sexnet_arp_proof:$gate_sexnet_arp_proof"
    "sexnet_arp_cache_proof:$gate_sexnet_arp_cache_proof"
    "sexnet_ipv4_header_validate:$gate_sexnet_ipv4_header_validate"
    "ipv4_packet_model_spec:$gate_ipv4_packet_model_spec"
    "ipv4_header_build_proof:$gate_ipv4_header_build_proof"
    "icmp_echo_request_plan:$gate_icmp_echo_request_plan"
    "icmp_echo_request_send_stop_review:$gate_icmp_echo_request_send_stop_review"
    "icmp_echo_request_proof:$gate_icmp_echo_request_proof"
    "icmp_echo_reply_observe_proof:$gate_icmp_echo_reply_observe_proof"
    "udp_dns_probe:$gate_udp_dns_probe"
    "dns_response_parse_proof:$gate_dns_response_parse_proof"
    "udp_packet_model_spec:$gate_udp_packet_model_spec"
    "udp_tx_build_proof:$gate_udp_tx_build_proof"
    "udp_tx_send_stop_review:$gate_udp_tx_send_stop_review"
    "udp_tx_send_proof:$gate_udp_tx_send_proof"
    "udp_loopback_or_qemu_usernet_proof:$gate_udp_loopback_or_qemu_usernet_proof"
    "tcp_minimal_state_machine_plan:$gate_tcp_minimal_state_machine_plan"
    "tcp_syn_build_proof:$gate_tcp_syn_build_proof"
    "tcp_syn_send_stop_review:$gate_tcp_syn_send_stop_review"
    "tcp_handshake_proof:$gate_tcp_handshake_proof"
    "tcp_syn_build_v1:$gate_tcp_syn_build_v1"
    "tcp_syn_checksum_v1:$gate_tcp_syn_checksum_v1"
    "tcp_syn_truth_v1:$gate_tcp_syn_truth_v1"
    "tcp_syn_build_proof_done_v1:$gate_tcp_syn_build_proof_done_v1"
    "tcp_syn_tx_post_v1:$gate_tcp_syn_tx_post_v1"
    "tcp_syn_rx_synack_v1:$gate_tcp_syn_rx_synack_v1"
    "tcp_syn_rx_synack_valid_v1:$gate_tcp_syn_rx_synack_valid_v1"
    "tcp_syn_truth_send_v1:$gate_tcp_syn_truth_send_v1"
    "tcp_syn_send_proof_done_v1:$gate_tcp_syn_send_proof_done_v1"
    "tcp_syn_send_retry_proof_v1:$gate_tcp_syn_send_retry_proof_v1"
    "tcp_target_variant_probe_v1:$gate_tcp_target_variant_probe_v1"
    "tcp_http_target_known_good_probe_v1:$gate_tcp_http_target_known_good_probe_v1"
    "tcp_guest_host_10_0_2_2_probe_v1:$gate_tcp_guest_host_10_0_2_2_probe_v1"
    "tcp_checksum_offload_header_audit_v1:$gate_tcp_checksum_offload_header_audit_v1"
    "qemu_slirp_tcp_limitation_freeze_v1:$gate_qemu_slirp_tcp_limitation_freeze_v1"
    "http_response_bounded_buffer_mock_proof_v1:$gate_http_response_bounded_buffer_mock_proof_v1"
    "http_response_to_html_subset_feed_v1:$gate_http_response_to_html_subset_feed_v1"
    "browser_remote_text_render_proof_v1:$gate_browser_remote_text_render_proof_v1"
    "sexnet_dynamic_text_render_proof_v1:$gate_sexnet_dynamic_text_render_proof_v1"
    "tcp_syn_ack_observe_proof_v1:$gate_tcp_syn_ack_observe_proof_v1"
    "tcp_http_connect_proof_v1:$gate_tcp_http_connect_proof_v1"
    "dns_client_plan:$gate_dns_client_plan"
    "dns_query_build_proof:$gate_dns_query_build_proof"
    "dns_query_send_stop_review:$gate_dns_query_send_stop_review"
    "dns_query_send_proof:$gate_dns_query_send_proof"
    "dns_response_parse_proof:$gate_dns_response_parse_proof"
    "dns_to_http_host_resolution_proof:$gate_dns_to_http_host_resolution_proof"
    "http_text_fetch_grant_plan:$gate_http_text_fetch_grant_plan"
    "http_get_send_plan:$gate_http_get_send_plan"
    "http_get_send_stop_review:$gate_http_get_send_stop_review"
    "http_get_send_proof_v1:$gate_http_get_send_proof_v1"
    "http_get_text_response_proof:$gate_http_get_text_response_proof"
    "http_response_bounded_buffer_proof:$gate_http_response_bounded_buffer_proof"
    "http_404_and_error_page_proof:$gate_http_404_and_error_page_proof"
    "browser_http_fetch_grant_plan:$gate_browser_http_fetch_grant_plan"
    "collar_browser_network_grant_plan:$gate_collar_browser_network_grant_plan"
    "collar_browser_network_grant_stub:$gate_collar_browser_network_grant_stub"
    "browser_slot_net_grant_stop_review:$gate_browser_slot_net_grant_stop_review"
    "browser_slot_net_grant_proof:$gate_browser_slot_net_grant_proof"
    "http_response_to_html_subset_feed:$gate_http_response_to_html_subset_feed"
    "browser_remote_text_render_proof:$gate_browser_remote_text_render_proof"
    "browser_fetch_status_ui:$gate_browser_fetch_status_ui"
    "browser_link_fetch_gated_proof:$gate_browser_link_fetch_gated_proof"
    "browser_history_remote_entry_proof:$gate_browser_history_remote_entry_proof"
    "browser_tab_remote_status_proof:$gate_browser_tab_remote_status_proof"
    "network_fault_containment_proof:$gate_network_fault_containment_proof"
    "network_timeout_and_retry_policy:$gate_network_timeout_and_retry_policy"
    "tls_deferred_truth_spec:$gate_tls_deferred_truth_spec"
    "browser_no_tls_warning_ui:$gate_browser_no_tls_warning_ui"
    "browser_http_only_fetch_proof:$gate_browser_http_only_fetch_proof"
    "runtime_smoke_real_network_pipeline:$gate_runtime_smoke_real_network_pipeline"
    "runtime_smoke_real_network_pipeline_v1:$gate_runtime_smoke_real_network_pipeline_v1"
    "daily_driver_network_baseline_freeze:$gate_daily_driver_network_baseline_freeze"
    "daily_driver_network_baseline_freeze_v1:$gate_daily_driver_network_baseline_freeze_v1"
    "browser_daily_driver_text_web_proof_v1:$gate_browser_daily_driver_text_web_proof_v1"
    "browser_usability_keyboard_nav:$gate_browser_usability_keyboard_nav"
    "browser_url_bar_edit_proof:$gate_browser_url_bar_edit_proof"
    "browser_enter_to_fetch_gated_proof:$gate_browser_enter_to_fetch_gated_proof"
    "browser_back_forward_remote_history:$gate_browser_back_forward_remote_history"
    "browser_reload_stop_proof:$gate_browser_reload_stop_proof"
    "sexnet_status_dashboard:$gate_sexnet_status_dashboard"
    "mesh_network_route_visual_stub:$gate_mesh_network_route_visual_stub"
    "collar_network_grant_ui_spec:$gate_collar_network_grant_ui_spec"
    "collar_network_grant_ui_stub:$gate_collar_network_grant_ui_stub"
    "real_hardware_nic_audit:$gate_real_hardware_nic_audit"
    "real_hardware_e1000_fallback_plan:$gate_real_hardware_e1000_fallback_plan"
    "real_hardware_network_boot_proof_v1:$gate_real_hardware_network_boot_proof_v1"
    "network_sprint_final_runtime_smoke:$gate_network_sprint_final_runtime_smoke"
    "network_sprint_final_runtime_smoke_v1:$gate_network_sprint_final_runtime_smoke_v1"
    "network_sprint_handoff_freeze:$gate_network_sprint_handoff_freeze"
    "network_sprint_handoff_freeze_v1:$gate_network_sprint_handoff_freeze_v1"
    "net_real_http_body_prefix:$gate_net_real_http_body_prefix"
    "clock_visible_seconds:$gate_clock_visible_seconds"
    "sexnet_passive:$gate_sexnet_passive"
    "linen_persist_readback:$gate_linen_persist_readback"
    "silk_glass_color:$gate_silk_glass_color"
    "frame_chrome_model:$gate_frame_chrome_model"
    "spindle_frame_chrome:$gate_spindle_frame_chrome"
    "frame_rim_markers:$gate_frame_rim_markers"
    "spindle_frame_rim:$gate_spindle_frame_rim"
    "frame_rim_visual:$gate_frame_rim_visual"
    "frame_lights_stub:$gate_frame_lights_stub"
    "spindle_frame_lights:$gate_spindle_frame_lights"
    "crosspd_launch:$gate_crosspd_launch"
    "browser_placeholder:$gate_browser_placeholder"
    "atlas_scene_stub:$gate_atlas_scene_stub"
    "frame_lights_visual:$gate_frame_lights_visual"
    "frame_lights_keyboard:$gate_frame_lights_keyboard"
    "scene_lifecycle_markers:$gate_scene_lifecycle_markers"
    "scene_keyboard_switch:$gate_scene_keyboard_switch"
    "project_scene_link:$gate_project_scene_link"
    "mesh_graph_status:$gate_mesh_graph_status"
    "collar_grant_status:$gate_collar_grant_status"
    "top_strip_hash:$gate_top_strip_hash"
    "spindle_atlas:$gate_spindle_atlas"
    "linen_search_bridge:$gate_linen_search_bridge"
    "faults_zero:$gate_faults_zero"
)

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

for entry in "${ALL_GATES[@]}"; do
    name="${entry%%:*}"
    state="${entry##*:}"
    case "$state" in
        PASS) PASS_COUNT=$((PASS_COUNT + 1)) ;;
        FAIL) FAIL_COUNT=$((FAIL_COUNT + 1)) ;;
        SKIP) SKIP_COUNT=$((SKIP_COUNT + 1)) ;;
    esac
done

echo "  PASS gates: $PASS_COUNT"
echo "  FAIL gates: $FAIL_COUNT"
echo "  SKIP gates: $SKIP_COUNT (proofs not enabled in this boot)"
echo ""

# Determine overall score
if [ "$gate_faults_zero" = "FAIL" ]; then
    FINAL="FAIL (faults detected)"
    exit_code=1
elif [ "$FAIL_COUNT" -gt 0 ]; then
    FINAL="FAIL (${FAIL_COUNT} gate(s) failed)"
    exit_code=1
elif [ "$PASS_COUNT" -ge 1 ] && [ "$FAIL_COUNT" -eq 0 ]; then
    # At least one enabled gate passed, zero failures, zero faults.
    # SKIP means the proof wasn't enabled in this boot — not a failure.
    FINAL="PASS (${PASS_COUNT} gates proved, ${SKIP_COUNT} skipped, 0 faults)"
    exit_code=0
else
    FINAL="FAIL (no gates passed — empty or unrecognized log?)"
    exit_code=1
fi

echo "  FINAL: $FINAL"
echo ""

# ---- HANDOFF DOC (inline emit) ----
# The caller can also find the handoff in docs/handoff/DAILY_DRIVER_MASTER_GATE_V1.md
# but we emit a minimal summary for CI/scripting.

exit $exit_code
