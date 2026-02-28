#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="$ROOT/build/qemu.log"
mkdir -p "$ROOT/build"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 missing; installing" >&2
  sudo apt-get update
  sudo apt-get install -y qemu-system-x86 xorriso make gcc mtools
fi

set +e
(timeout 40 "$ROOT/scripts/run_qemu_headless.sh" >"$LOG" 2>&1)
status=$?
set -e

if [[ $status -eq 124 ]]; then
  echo "QEMU timed out" >&2
  cat "$LOG"
  exit 1
fi

if [[ $status -ne 33 ]]; then
  echo "QEMU exited with unexpected status $status" >&2
  cat "$LOG"
  exit 1
fi

rg -q "\[kernel\] limine boot ok" "$LOG"
rg -q "\[init\] hello from userspace" "$LOG"
rg -q "\[init\] done" "$LOG"

echo "qemu smoke OK"
