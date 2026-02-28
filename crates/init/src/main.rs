#![no_std]
#![no_main]

use common::syscall::{FD_STDIN, FD_STDOUT};
use core::arch::asm;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let _ = mlibc::write(
        FD_STDOUT,
        b"[init] hello from userspace via write() syscall\n",
    );

    if let Ok(mapped) = mlibc::memmap(16 * 1024) {
        let _ = mlibc::write(FD_STDOUT, b"[init] memmap ok at ");
        write_hex(mapped as usize);
        let _ = mlibc::write(FD_STDOUT, b"\n");
    }

    let _ = mlibc::write(FD_STDOUT, b"[init] type one line and press enter:\n");
    let mut buf = [0u8; 64];
    let n = mlibc::read(FD_STDIN, &mut buf).unwrap_or(0);
    let _ = mlibc::write(FD_STDOUT, b"[init] echo: ");
    let _ = mlibc::write(FD_STDOUT, &buf[..n]);
    let _ = mlibc::write(FD_STDOUT, b"[init] done\n");

    unsafe {
        asm!("out dx, al", in("dx") 0xF4u16, in("al") 0x10u8, options(nostack, nomem));
    }

    loop {
        unsafe { asm!("hlt") };
    }
}

fn write_hex(mut v: usize) {
    let mut out = [0u8; 2 + 16];
    out[0] = b'0';
    out[1] = b'x';
    for i in (2..18).rev() {
        let nib = (v & 0xF) as u8;
        out[i] = if nib < 10 {
            b'0' + nib
        } else {
            b'a' + (nib - 10)
        };
        v >>= 4;
    }
    let _ = mlibc::write(FD_STDOUT, &out);
}
