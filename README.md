# Tiny Limine OS (x86_64)

This repository now contains a **real bootable prototype OS**:

- Boots with **Limine** on x86_64.
- Runs a tiny single-task kernel.
- Reads a **USTAR initramfs** module and locates `init.elf`.
- Parses ELF64 and transfers control to `init.elf`.
- Exposes three syscall numbers (`read`, `write`, `memmap`) and Unix-like fd values (`stdin=0`, `stdout=1`).
- Includes headless QEMU automation scripts/tests.

## Layout

- `crates/common`: shared ABI + USTAR/ELF parsers.
- `crates/kernel`: no_std kernel entry, ELF loading, IDT/syscall setup, serial output, and memory manager.
- `crates/init`: no_std Rust-only user init program with direct syscall wrappers (no libc layer).
- `scripts/`: image build + QEMU run harness.
- `tests/`: host + headless smoke checks.

## Quickstart

```bash
cargo test -p common
./scripts/build_image.sh
./scripts/run_qemu_headless.sh
```

## Note

This was made 100% by AI, if you don't like that, check out [Boonix](https://github.com/laurinyev/boonix)!

## License

AGPL-3.0-only.
