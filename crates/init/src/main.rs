#![no_std]
#![no_main]

use common::syscall::{FD_STDIN, FD_STDOUT};
use core::arch::asm;

mod syscall;

const SPAWN_TARGET: &str = if cfg!(feature = "test-build") {
    "testbin.elf"
} else {
    "/bin/shell.elf"
};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let _ = syscall::write(
        FD_STDOUT,
        b"[init] hello from userspace via write() syscall\n",
    );

    if let Ok(mapped) = syscall::memmap(16 * 1024) {
        let _ = syscall::write(FD_STDOUT, b"[init] memmap ok at ");
        write_hex(mapped as usize);
        let _ = syscall::write(FD_STDOUT, b"\n");
    }

    print_motd();

    let _ = syscall::write(FD_STDOUT, b"[init] trying stack-process fork()\n");
    let pid = syscall::fork(parent_resume as usize).unwrap_or(0);
    if pid == 0 {
        let _ = syscall::write(FD_STDOUT, b"[init] child process is now running\n");
        let _ = syscall::write(FD_STDOUT, b"[init] child execve target: ");
        let _ = syscall::write(FD_STDOUT, SPAWN_TARGET.as_bytes());
        let _ = syscall::write(FD_STDOUT, b"\n");
        let _ = syscall::execve(SPAWN_TARGET);
        let _ = syscall::write(FD_STDOUT, b"[init] child execve failed, exiting\n");
        let _ = syscall::exit(1);
    }

    parent_resume()
}

extern "C" fn parent_resume() -> ! {
    let _ = syscall::write(FD_STDOUT, b"[init] parent resumed after child exit\n");
    interaction_and_shutdown()
}

fn print_motd() {
    match syscall::open("motd.txt") {
        Ok(fd) => {
            let _ = syscall::write(FD_STDOUT, b"[init] motd: ");
            let mut buf = [0u8; 64];
            loop {
                let n = syscall::read(fd, &mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                let _ = syscall::write(FD_STDOUT, &buf[..n]);
            }
        }
        Err(_) => {
            let _ = syscall::write(FD_STDOUT, b"[init] motd unavailable\n");
        }
    }
}

fn interaction_and_shutdown() -> ! {
    let _ = syscall::write(FD_STDOUT, b"[init] type one line and press enter:\n");
    let mut buf = [0u8; 64];
    let n = syscall::read(FD_STDIN, &mut buf).unwrap_or(0);
    let _ = syscall::write(FD_STDOUT, b"[init] echo: ");
    let _ = syscall::write(FD_STDOUT, &buf[..n]);
    let _ = syscall::write(FD_STDOUT, b"[init] done\n");

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
    let _ = syscall::write(FD_STDOUT, &out);
}
