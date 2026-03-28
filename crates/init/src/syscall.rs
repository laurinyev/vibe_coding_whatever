use core::arch::asm;

use common::syscall::{SYS_EXECVE, SYS_EXIT, SYS_FORK, SYS_MEMMAP, SYS_OPEN, SYS_READ, SYS_WRITE};

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

pub fn write(fd: u64, bytes: &[u8]) -> Result<usize, isize> {
    let ret = syscall3(SYS_WRITE, fd, bytes.as_ptr() as u64, bytes.len() as u64);
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}

pub fn read(fd: u64, bytes: &mut [u8]) -> Result<usize, isize> {
    let ret = syscall3(SYS_READ, fd, bytes.as_mut_ptr() as u64, bytes.len() as u64);
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}

pub fn open(path: &str) -> Result<u64, isize> {
    let ret = syscall3(SYS_OPEN, path.len() as u64, path.as_ptr() as u64, 0);
    if ret < 0 { Err(ret) } else { Ok(ret as u64) }
}

pub fn memmap(length: usize) -> Result<*mut u8, isize> {
    let ret = syscall3(SYS_MEMMAP, length as u64, 0, 0);
    if ret < 0 {
        Err(ret)
    } else {
        Ok(ret as usize as *mut u8)
    }
}

pub fn fork(parent_resume_rip: usize) -> Result<usize, isize> {
    let ret = syscall3(SYS_FORK, 0, parent_resume_rip as u64, 0);
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}

pub fn execve(path: &str) -> Result<usize, isize> {
    let ret = syscall3(SYS_EXECVE, path.len() as u64, path.as_ptr() as u64, 0);
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}

pub fn exit(code: i32) -> Result<usize, isize> {
    let ret = syscall3(SYS_EXIT, code as u64, 0, 0);
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}
