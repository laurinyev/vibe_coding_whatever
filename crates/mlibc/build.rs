use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn ensure_mlibc_checkout(out_dir: &Path) -> PathBuf {
    let checkout = out_dir.join("mlibc-src");
    if checkout.exists() {
        return checkout;
    }

    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/managarm/mlibc.git",
            checkout.to_str().expect("utf8 path"),
        ])
        .status()
        .expect("failed to spawn git clone for mlibc");

    assert!(status.success(), "failed to clone mlibc");
    checkout
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let mlibc_src = ensure_mlibc_checkout(&out_dir);

    println!("cargo:rerun-if-changed=src/port/mlibc_port.c");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!(
        "cargo:warning=building mlibc shim against checkout at {}",
        mlibc_src.display()
    );

    cc::Build::new()
        .compiler("cc")
        .file("src/port/mlibc_port.c")
        .flag("-ffreestanding")
        .flag("-fno-stack-protector")
        .flag("-nostdlib")
        .flag("-fno-builtin")
        .compile("mlibc_port");
}
