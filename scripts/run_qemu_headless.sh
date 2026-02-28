#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO="$ROOT/build/os.iso"

if [[ ! -f "$ISO" ]]; then
  "$ROOT/scripts/build_image.sh"
fi

qemu-system-x86_64 \
  -m 256M \
  -cdrom "$ISO" \
  -boot d \
  -display none \
  -no-reboot \
  -no-shutdown \
  -serial none \
  -debugcon stdio \
  -global isa-debugcon.iobase=0xe9 \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04
