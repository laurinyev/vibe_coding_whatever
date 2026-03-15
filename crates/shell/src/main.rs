#![no_std]
#![no_main]

use common::syscall::{FD_STDIN, FD_STDOUT, SYS_READ, SYS_WRITE};
use core::arch::asm;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

fn syscall3(n: u64, a: u64, b: u64, c: u64) -> isize {
    let ret: i64;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") n,
            in("rdi") a,
            in("rsi") b,
            in("rdx") c,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret as isize
}

fn write(bytes: &[u8]) {
    let _ = syscall3(
        SYS_WRITE,
        FD_STDOUT,
        bytes.as_ptr() as u64,
        bytes.len() as u64,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    write(b"[shell] tiny shell started\n");
    write(b"shell> ");
    let mut buf = [0u8; 64];
    let n = syscall3(
        SYS_READ,
        FD_STDIN,
        buf.as_mut_ptr() as u64,
        buf.len() as u64,
    );
    if n > 0 {
        write(b"[shell] got: ");
        write(&buf[..n as usize]);
    }
    write(b"[shell] done\n");

    loop {
        unsafe { asm!("hlt") };
    }
}
