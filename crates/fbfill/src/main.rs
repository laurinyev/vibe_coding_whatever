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

fn write_stdout(bytes: &[u8]) {
    let _ = syscall3(SYS_WRITE, 1, bytes.as_ptr() as u64, bytes.len() as u64);
}

fn parse_u64_le(bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(bytes);
    u64::from_le_bytes(arr)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let path = "/dev/fb0";
    let fd = syscall3(SYS_OPEN, path.len() as u64, path.as_ptr() as u64, 0);
    if fd < 0 {
        write_stdout(b"[fbfill] open /dev/fb0 failed\n");
        let _ = syscall3(SYS_EXIT, 1, 0, 0);
        loop {
            unsafe { asm!("hlt") };
        }
    }

    let mut header = [0u8; 32];
    let n = syscall3(
        SYS_READ,
        fd as u64,
        header.as_mut_ptr() as u64,
        header.len() as u64,
    );
    if n != 32 {
        write_stdout(b"[fbfill] failed to read fb header\n");
        let _ = syscall3(SYS_EXIT, 1, 0, 0);
        loop {
            unsafe { asm!("hlt") };
        }
    }

    let height = parse_u64_le(&header[8..16]) as usize;
    let pitch = parse_u64_le(&header[16..24]) as usize;
    let bytes_per_pixel = parse_u64_le(&header[24..32]) as usize;

    if height == 0 || pitch == 0 || bytes_per_pixel == 0 {
        write_stdout(b"[fbfill] invalid fb geometry\n");
        let _ = syscall3(SYS_EXIT, 1, 0, 0);
        loop {
            unsafe { asm!("hlt") };
        }
    }

    let total = pitch.saturating_mul(height);
    let mut buf = [0u8; 4096];
    let mut filled = 0usize;

    while filled < total {
        let chunk = core::cmp::min(buf.len(), total - filled);
        let pix = core::cmp::max(1, bytes_per_pixel);

        let mut i = 0usize;
        while i < chunk {
            buf[i] = 0xff;
            if i + 1 < chunk {
                buf[i + 1] = 0x00;
            }
            if i + 2 < chunk {
                buf[i + 2] = 0x00;
            }
            if pix >= 4 && i + 3 < chunk {
                buf[i + 3] = 0x00;
            }
            let mut j = 4usize;
            while j < pix && i + j < chunk {
                buf[i + j] = 0x00;
                j += 1;
            }
            i += pix;
        }

        let wrote = syscall3(SYS_WRITE, fd as u64, buf.as_ptr() as u64, chunk as u64);
        if wrote <= 0 {
            write_stdout(b"[fbfill] framebuffer write failed\n");
            let _ = syscall3(SYS_EXIT, 1, 0, 0);
            loop {
                unsafe { asm!("hlt") };
            }
        }
        filled += wrote as usize;
    }

    write_stdout(b"[fbfill] filled framebuffer with blue\n");
    let _ = syscall3(SYS_EXIT, 0, 0, 0);
    loop {
        unsafe { asm!("hlt") };
    }
}
