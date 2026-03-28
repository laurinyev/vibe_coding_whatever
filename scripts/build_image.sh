#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD="$ROOT/build"
ISO="$BUILD/os.iso"
LIMINE_DIR="$BUILD/limine"
INIT_FEATURES="${INIT_FEATURES:-}"

mkdir -p "$BUILD/root/boot"

cargo build --manifest-path "$ROOT/crates/kernel/Cargo.toml" --release --target x86_64-unknown-none
if [[ -n "$INIT_FEATURES" ]]; then
  cargo build --manifest-path "$ROOT/crates/init/Cargo.toml" --release --target x86_64-unknown-none --features "$INIT_FEATURES"
else
  cargo build --manifest-path "$ROOT/crates/init/Cargo.toml" --release --target x86_64-unknown-none
fi
cargo build --manifest-path "$ROOT/crates/testbin/Cargo.toml" --release --target x86_64-unknown-none
cargo build --manifest-path "$ROOT/crates/shell/Cargo.toml" --release --target x86_64-unknown-none
cargo build --manifest-path "$ROOT/crates/fbfill/Cargo.toml" --release --target x86_64-unknown-none

cp "$ROOT/target/x86_64-unknown-none/release/kernel" "$BUILD/root/boot/kernel"
cp "$ROOT/target/x86_64-unknown-none/release/init" "$BUILD/init.elf"
mkdir -p "$BUILD/bin"
cp "$ROOT/target/x86_64-unknown-none/release/testbin" "$BUILD/testbin.elf"
cp "$ROOT/target/x86_64-unknown-none/release/shell" "$BUILD/shell.elf"
cp "$ROOT/target/x86_64-unknown-none/release/fbfill" "$BUILD/fbfill.elf"
cp "$ROOT/target/x86_64-unknown-none/release/testbin" "$BUILD/bin/testbin.elf"
cp "$ROOT/target/x86_64-unknown-none/release/shell" "$BUILD/bin/shell.elf"
cp "$ROOT/target/x86_64-unknown-none/release/fbfill" "$BUILD/bin/fbfill.elf"
printf "hello-from-initrd\n" > "$BUILD/test.txt"
printf "Welcome to PromptOS - 100%% certified vibecoded.\n" > "$BUILD/motd.txt"

( cd "$BUILD" && tar --format=ustar -cf initramfs.tar init.elf testbin.elf shell.elf fbfill.elf test.txt motd.txt bin/testbin.elf bin/shell.elf bin/fbfill.elf )
cp "$BUILD/initramfs.tar" "$BUILD/root/boot/initramfs.tar"
cp "$ROOT/limine.conf" "$BUILD/root/boot/limine.conf"

if [[ ! -d "$LIMINE_DIR" ]]; then
  git clone --depth 1 --branch v10.x-binary https://github.com/limine-bootloader/limine.git "$LIMINE_DIR"
fi

if [[ ! -x "$LIMINE_DIR/limine" ]]; then
  make -C "$LIMINE_DIR"
fi

cp "$LIMINE_DIR"/limine-bios.sys "$BUILD/root/boot/"
cp "$LIMINE_DIR"/limine-bios-cd.bin "$BUILD/root/boot/"
cp "$LIMINE_DIR"/limine-uefi-cd.bin "$BUILD/root/boot/"

xorriso -as mkisofs \
  -b boot/limine-bios-cd.bin \
  -no-emul-boot -boot-load-size 4 -boot-info-table \
  --efi-boot boot/limine-uefi-cd.bin \
  -efi-boot-part --efi-boot-image --protective-msdos-label \
  "$BUILD/root" -o "$ISO"

"$LIMINE_DIR"/limine bios-install "$ISO"

echo "Built $ISO"
