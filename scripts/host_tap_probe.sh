#!/bin/bash
set -euo pipefail

function print_setup_commands() {
    echo "TAP_SETUP_NEEDED"
    echo ""
    echo "To setup TAP/TUN networking for SexOS, run the following commands:"
    echo "----------------------------------------------------------------"
    echo "sudo modprobe tun"
    echo "sudo mkdir -p /dev/net"
    echo "sudo mknod /dev/net/tun c 10 200 || true"
    echo "sudo chmod 666 /dev/net/tun"
    echo "sudo ip tuntap add dev tap0 mode tap user \"\$USER\""
    echo "sudo ip addr flush dev tap0 || true"
    echo "sudo ip addr add 10.0.2.2/24 dev tap0"
    echo "sudo ip link set tap0 up"
    echo "----------------------------------------------------------------"
    echo "Then re-run this script."
}

function check_tap() {
    if [ ! -c /dev/net/tun ]; then
        echo "Error: /dev/net/tun is missing or not a character device."
        print_setup_commands
        exit 1
    fi

    if ! ip link show tap0 >/dev/null 2>&1; then
        echo "Error: interface tap0 does not exist."
        print_setup_commands
        exit 1
    fi

    # TAP devices often report operstate DOWN/UNKNOWN until QEMU attaches.
    # For host setup readiness, administrative UP + configured address is sufficient.
    if ! ip link show dev tap0 2>/dev/null | head -n1 | grep -q "<[^>]*UP"; then
        echo "Error: interface tap0 is not administratively UP."
        print_setup_commands
        exit 1
    fi

    if ! ip addr show tap0 2>/dev/null | grep -q "10.0.2.2/24"; then
        echo "Error: interface tap0 does not have expected address 10.0.2.2/24."
        print_setup_commands
        exit 1
    fi

    echo "Note: TAP operstate may remain DOWN/UNKNOWN until QEMU attaches; admin UP is sufficient."

    echo "PASS"
}

check_tap
