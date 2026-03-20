#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="$ROOT/build/qemu.log"
mkdir -p "$ROOT/build"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 missing; installing" >&2
  apt-get update
  apt-get install -y qemu-system-x86 xorriso make gcc mtools
fi

INIT_FEATURES=test-build "$ROOT/scripts/build_image.sh"

set +e
python3 - "$ROOT" "$LOG" <<'PY'
import os
import pty
import select
import signal
import subprocess
import sys
import time

root, log_path = sys.argv[1], sys.argv[2]
master_fd, slave_fd = pty.openpty()

with open(log_path, "wb") as log:
    proc = subprocess.Popen(
        [f"{root}/scripts/run_qemu_headless.sh"],
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=slave_fd,
        cwd=root,
        preexec_fn=os.setsid,
    )

    os.close(slave_fd)
    deadline = time.monotonic() + 40
    sent_input = False
    status = 1
    recent = b""

    try:
        while True:
            if time.monotonic() > deadline:
                os.killpg(proc.pid, signal.SIGTERM)
                status = 124
                break

            ready, _, _ = select.select([master_fd], [], [], 0.2)
            if master_fd in ready:
                try:
                    data = os.read(master_fd, 4096)
                except OSError:
                    data = b""
                if data:
                    log.write(data)
                    log.flush()
                    recent = (recent + data)[-8192:]
                    if (not sent_input) and b"[init] type one line and press enter:" in recent:
                        os.write(master_fd, b"smoke-input\r")
                        sent_input = True

            ret = proc.poll()
            if ret is not None:
                status = ret
                break
    finally:
        try:
            os.close(master_fd)
        except OSError:
            pass

sys.exit(status)
PY
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
rg -q "\[kernel\] process stack ready" "$LOG"
rg -q "\[kernel\] fork: pushed child pid=" "$LOG"
rg -q "\[init\] motd: Welcome to PromptOS - 100% certified vibecoded." "$LOG"
rg -q "\[init\] child process is now running" "$LOG"
rg -q "\[init\] child execve target: testbin.elf" "$LOG"
rg -q "\[testbin\] hello from execve target" "$LOG"
rg -q "\[testbin\] read test.txt: hell" "$LOG"
rg -q "\[kernel\] execve: replaced current process image with testbin.elf" "$LOG"
rg -q "\[kernel\] exit\(0\): popped current process" "$LOG"
rg -q "\[init\] parent resumed after child exit" "$LOG"
rg -q "\[init\] echo: smoke-input" "$LOG"
rg -q "\[init\] done" "$LOG"

echo "qemu smoke OK"
