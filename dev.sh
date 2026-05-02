#!/usr/bin/env bash
set -e

CMD="${1:-run}"

case "$CMD" in
  run)
    qemu-system-x86_64 \
      -M q35 \
      -m 512M \
      -cpu max,+pku \
      -cdrom sexos-v1.0.0.iso \
      -serial stdio \
      -display sdl
    ;;
  run-nographic)
    qemu-system-x86_64 \
      -M q35 \
      -m 512M \
      -cpu max,+pku \
      -cdrom sexos-v1.0.0.iso \
      -serial stdio \
      -nographic
    ;;
  *)
    echo "usage: ./dev.sh [run|run-nographic]"
    exit 1
    ;;
esac
