#![no_std]
#![no_main]

use core::arch::asm;
use common::syscall::{FD_STDIN, FD_STDOUT, SYS_READ, SYS_WRITE};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let banner = b"[init] hello from userspace via write() syscall\n";
    let _ = sys_write(FD_STDOUT, banner);

    let _ = sys_write(FD_STDOUT, b"[init] type one line and press enter:\n");
    let mut buf = [0u8; 64];
    let n = sys_read(FD_STDIN, &mut buf).unwrap_or(0);
    let _ = sys_write(FD_STDOUT, b"[init] echo: ");
    let _ = sys_write(FD_STDOUT, &buf[..n]);
    let _ = sys_write(FD_STDOUT, b"[init] done\n");

    unsafe {
        asm!("out dx, al", in("dx") 0xF4u16, in("al") 0x10u8, options(nostack, nomem));
    }

    loop {
        unsafe { asm!("hlt") };
    }
}

fn sys_write(fd: u64, bytes: &[u8]) -> Result<usize, isize> {
    syscall3(SYS_WRITE, fd, bytes.as_ptr() as u64, bytes.len() as u64)
}

fn sys_read(fd: u64, bytes: &mut [u8]) -> Result<usize, isize> {
    syscall3(SYS_READ, fd, bytes.as_mut_ptr() as u64, bytes.len() as u64)
}

fn syscall3(n: u64, a: u64, b: u64, c: u64) -> Result<usize, isize> {
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
    if ret < 0 {
        Err(ret as isize)
    } else {
        Ok(ret as usize)
    }
}
