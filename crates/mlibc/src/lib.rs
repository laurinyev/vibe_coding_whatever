#![no_std]

use common::syscall::{SYS_MEMMAP, SYS_READ, SYS_WRITE};
use core::arch::asm;

pub fn write(fd: u64, bytes: &[u8]) -> Result<usize, isize> {
    syscall3(SYS_WRITE, fd, bytes.as_ptr() as u64, bytes.len() as u64)
}

pub fn read(fd: u64, bytes: &mut [u8]) -> Result<usize, isize> {
    syscall3(SYS_READ, fd, bytes.as_mut_ptr() as u64, bytes.len() as u64)
}

pub fn memmap(length: usize) -> Result<*mut u8, isize> {
    syscall3(SYS_MEMMAP, length as u64, 0, 0).map(|addr| addr as *mut u8)
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
