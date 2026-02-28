fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    println!("cargo:rustc-link-arg=-T{dir}/linker.ld");
}
