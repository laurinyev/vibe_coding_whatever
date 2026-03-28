#![no_std]
#![no_main]

use common::syscall::{FD_STDIN, FD_STDOUT, SYS_EXECVE, SYS_EXIT, SYS_FORK, SYS_READ, SYS_WRITE};
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
    let _ = syscall3(
        SYS_WRITE,
        FD_STDOUT,
        bytes.as_ptr() as u64,
        bytes.len() as u64,
    );
}

fn read_line(buf: &mut [u8]) -> usize {
    let n = syscall3(SYS_READ, FD_STDIN, buf.as_mut_ptr() as u64, buf.len() as u64);
    if n <= 0 {
        return 0;
    }
    n as usize
}

fn first_word(line: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < line.len()
        && (line[start] == b' '
            || line[start] == b'\t'
            || line[start] == b'\n'
            || line[start] == b'\r')
    {
        start += 1;
    }
    let mut end = start;
    while end < line.len()
        && line[end] != b' '
        && line[end] != b'\t'
        && line[end] != b'\n'
        && line[end] != b'\r'
    {
        end += 1;
    }
    &line[start..end]
}

fn build_exec_path<'a>(cmd: &[u8], out: &'a mut [u8]) -> Option<&'a str> {
    let prefix = b"/bin/";
    let suffix = b".elf";
    let total = prefix.len() + cmd.len() + suffix.len();
    if total > out.len() {
        return None;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..prefix.len() + cmd.len()].copy_from_slice(cmd);
    out[prefix.len() + cmd.len()..total].copy_from_slice(suffix);
    core::str::from_utf8(&out[..total]).ok()
}

fn shell_loop() -> ! {
    let mut line = [0u8; 64];
    let mut exec_path_buf = [0u8; 96];

    loop {
        write(b"shell> ");
        let used = read_line(&mut line);
        let word = first_word(&line[..used]);
        if word.is_empty() {
            continue;
        }

        if word == b"exit" {
            write(b"[shell] bye\n");
            let _ = syscall3(SYS_EXIT, 0, 0, 0);
            loop {
                unsafe { asm!("hlt") };
            }
        }

        let Some(path) = build_exec_path(word, &mut exec_path_buf) else {
            write(b"[shell] command too long\n");
            continue;
        };

        write(b"[shell] exec: ");
        write(path.as_bytes());
        write(b"\n");

        let pid = syscall3(SYS_FORK, 0, shell_resume as usize as u64, 0);
        if pid == 0 {
            let _ = syscall3(SYS_EXECVE, path.len() as u64, path.as_ptr() as u64, 0);
            write(b"[shell] exec failed: ");
            write(path.as_bytes());
            write(b"\n");
            let _ = syscall3(SYS_EXIT, 127, 0, 0);
            loop {
                unsafe { asm!("hlt") };
            }
        }
    }
}

extern "C" fn shell_resume() -> ! {
    shell_loop()
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    write(b"[shell] tiny shell started\n");
    shell_loop()
}
