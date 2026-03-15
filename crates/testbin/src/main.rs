#![no_std]
#![no_main]

use common::syscall::{FD_STDOUT, SYS_EXIT, SYS_WRITE};
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
    write(b"[testbin] hello from execve target\n");
    let _ = syscall3(SYS_EXIT, 0, 0, 0);

    loop {
        unsafe { asm!("hlt") };
    }
}
