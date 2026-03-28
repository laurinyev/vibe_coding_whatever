#![no_std]
#![no_main]

use common::syscall::{SYS_EXIT, SYS_OPEN, SYS_READ, SYS_WRITE};
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
            lateout("rcx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret as isize
}

fn write(bytes: &[u8]) {
    let _ = syscall3(SYS_WRITE, 1, bytes.as_ptr() as u64, bytes.len() as u64);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    write(b"[testbin] hello from execve target\n");

    let path = "test.txt";
    let fd = syscall3(SYS_OPEN, path.len() as u64, path.as_ptr() as u64, 0);
    if fd >= 0 {
        let mut buf = [0u8; 4];
        let n = syscall3(
            SYS_READ,
            fd as u64,
            buf.as_mut_ptr() as u64,
            buf.len() as u64,
        );
        if n > 0 {
            write(b"[testbin] read test.txt: ");
            write(&buf[..n as usize]);
        }
    } else {
        write(b"[testbin] open test.txt failed\n");
    }

    let _ = syscall3(SYS_EXIT, 0, 0, 0);

    loop {
        unsafe { asm!("hlt") };
    }
}
