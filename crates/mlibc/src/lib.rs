#![no_std]

use core::arch::asm;
use core::ffi::c_void;

unsafe extern "C" {
    fn mlibc_sys_write(fd: i32, buf: *const u8, len: usize) -> isize;
    fn mlibc_sys_read(fd: i32, buf: *mut u8, len: usize) -> isize;
    fn mlibc_sys_memmap(len: usize) -> *mut u8;
}

const SYS_READ: u64 = 0;
const SYS_WRITE: u64 = 1;
const SYS_MEMMAP: u64 = 9;

#[unsafe(no_mangle)]
extern "C" fn __mlibc_rs_write(fd: i32, buf: *const c_void, len: usize) -> isize {
    syscall3(SYS_WRITE, fd as u64, buf as u64, len as u64)
}

#[unsafe(no_mangle)]
extern "C" fn __mlibc_rs_read(fd: i32, buf: *mut c_void, len: usize) -> isize {
    syscall3(SYS_READ, fd as u64, buf as u64, len as u64)
}

#[unsafe(no_mangle)]
extern "C" fn __mlibc_rs_memmap(len: usize) -> *mut u8 {
    let r = syscall3(SYS_MEMMAP, len as u64, 0, 0);
    if r < 0 {
        core::ptr::null_mut()
    } else {
        r as usize as *mut u8
    }
}

pub fn write(fd: u64, bytes: &[u8]) -> Result<usize, isize> {
    let ret = unsafe { mlibc_sys_write(fd as i32, bytes.as_ptr(), bytes.len()) };
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}

pub fn read(fd: u64, bytes: &mut [u8]) -> Result<usize, isize> {
    let ret = unsafe { mlibc_sys_read(fd as i32, bytes.as_mut_ptr(), bytes.len()) };
    if ret < 0 { Err(ret) } else { Ok(ret as usize) }
}

pub fn memmap(length: usize) -> Result<*mut u8, isize> {
    let ptr = unsafe { mlibc_sys_memmap(length) };
    if ptr.is_null() { Err(-12) } else { Ok(ptr) }
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
