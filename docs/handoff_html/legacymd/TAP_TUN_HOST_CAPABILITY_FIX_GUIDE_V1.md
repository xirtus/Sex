# TAP_TUN_HOST_CAPABILITY_FIX_GUIDE_V1

## Current Truth
SexOS packet path is good enough: e1000e TX/RX, ARP, ICMP, UDP DNS, and TCP SYN TX/checksums are proven. 
However, the host backend is blocked. QEMU user/SLiRP gives no TCP SYN-ACK/RST, and `hostfwd` cannot set the necessary forwarding rules. TAP cannot run because `/dev/net/tun` is missing or not configured correctly on the host.

## Diagnostics

1. Check for `/dev/net/tun`:
    ```bash
    ls -l /dev/net/tun || true
    lsmod | grep tun || true
    ```

## Tun Enablement

Run the following commands on the Linux host to enable TUN/TAP support:
```bash
sudo modprobe tun
sudo mkdir -p /dev/net
sudo mknod /dev/net/tun c 10 200
sudo chmod 666 /dev/net/tun
```

## tap0 Setup

*Note: Current SexOS static guest config expects 10.0.2.0/24 subnet. The preferred TAP host address for the current proof is 10.0.2.2/24. The previously used 10.0.3.1/24 will only work after guest net config becomes configurable.*

Create and configure the `tap0` interface:
```bash
sudo ip tuntap add dev tap0 mode tap user "$USER"
sudo ip addr flush dev tap0 || true
sudo ip addr add 10.0.2.2/24 dev tap0
sudo ip link set tap0 up
```

## Optional Cleanup

To remove the tap interface later:
```bash
sudo ip link set tap0 down || true
sudo ip tuntap del dev tap0 mode tap || true
```

## QEMU Proof Command

Once TAP is configured, test the network boot with:
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexos_tap_network_boot.log
```

Next mission name only if TAP boots: **TAP_TCP_SYNACK_PROOF_V1**
