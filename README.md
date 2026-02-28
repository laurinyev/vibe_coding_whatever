# Tiny Limine OS (x86_64)

This repository now contains a **real bootable prototype OS**:

- Boots with **Limine** on x86_64.
- Runs a tiny single-task kernel.
- Reads a **USTAR initramfs** module and locates `init.elf`.
- Parses ELF64 and transfers control to `init.elf`.
- Exposes two syscall numbers (`read`, `write`) and Unix-like fd values (`stdin=0`, `stdout=1`).
- Includes headless QEMU automation scripts/tests.

## Layout

- `crates/common`: shared ABI + USTAR/ELF parsers.
- `crates/kernel`: no_std kernel entry and syscall handling.
- `crates/init`: no_std user init program using the mlibc compatibility layer.
- `crates/mlibc`: tiny mlibc-compat shim implementing read/write/memmap syscall wrappers.
- `scripts/`: image build + QEMU run harness.
- `tests/`: host + headless smoke checks.

## Quickstart

```bash
cargo test -p common
./scripts/build_image.sh
./scripts/run_qemu_headless.sh
```

## License

AGPL-3.0-only.
