# PromptOS (x86_64)

This repository now contains a **real bootable prototype OS**:

- Boots with **Limine** on x86_64.
- Runs a tiny single-task kernel.
- Reads a **USTAR initramfs** module and locates `init.elf`.
- Parses ELF64 and transfers control to `init.elf`.
- Exposes syscalls for `read`, `write`, `memmap`, `fork`, `execve`, `exit`, and `open` with Unix-like fd values (`stdin=0`, `stdout=1`).
- Includes headless QEMU automation scripts/tests.

## Layout

- `crates/common`: shared ABI + USTAR/ELF parsers.
- `crates/kernel`: no_std kernel entry, ELF loading, IDT/syscall setup, serial output, memory manager, and a stack-based process system.
- `crates/init`: no_std Rust-only user init program with direct syscall wrappers (no libc layer).
- `crates/testbin`: tiny no_std exec target used by init/shell to validate fork+execve+exit/open behavior (including reading `test.txt` from initrd).
- `crates/shell`: tiny no_std shell-like exec target used by non-test init builds, launched as `/bin/shell.elf` by init.
- `scripts/`: image build + QEMU run harness.
- `tests/`: host + headless smoke checks.

## Quickstart

```bash
cargo test -p common
./scripts/build_image.sh
./scripts/run_qemu_headless.sh
# smoke path uses: INIT_FEATURES=test-build ./scripts/build_image.sh
```

## Note

This was made 100% by AI, if you don't like that, check out [Boonix](https://github.com/laurinyev/boonix)!

## License

AGPL-3.0-only.
