#![no_std]
#![no_main]

mod elf_loader;
mod interrupts;
mod memory;
mod serial;
mod tty;
mod vfs;

use common::elf::parse_elf64;
use common::process::ProcessStack;
use common::syscall::{SYS_EXECVE, SYS_EXIT, SYS_FORK, SYS_MEMMAP, SYS_OPEN, SYS_READ, SYS_WRITE};
use common::ustar::find_file;
use core::arch::asm;
use core::fmt::Write;
use limine::BaseRevision;
use limine::request::{FramebufferRequest, ModuleRequest, RequestsEndMarker, RequestsStartMarker};
use spin::Mutex;
use tty::TTY;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
#[used]
#[unsafe(link_section = ".requests")]
static MODULE_REQUEST: ModuleRequest = ModuleRequest::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    let _ = writeln!(TTY.lock(), "[panic] {info}");
    loop {
        unsafe { asm!("hlt") };
    }
}

static PROCESS_STACK: Mutex<ProcessStack<16>> = Mutex::new(ProcessStack::new());

static mut INITRAMFS_ADDR: usize = 0;
static mut INITRAMFS_SIZE: usize = 0;

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    serial::serial_init();

    if let Some(resp) = FRAMEBUFFER_REQUEST.get_response()
        && let Some(fb) = resp.framebuffers().next()
    {
        tty::set_framebuffer(fb);
    }

    let _ = writeln!(TTY.lock(), "[kernel] limine boot ok");
    let _ = writeln!(
        TTY.lock(),
        "[kernel] simple mem manager ready ({} KiB)",
        memory::USER_MEM_POOL_SIZE / 1024
    );

    interrupts::install_idt();

    let module = MODULE_REQUEST
        .get_response()
        .and_then(|m| m.modules().first().copied())
        .expect("missing initramfs module");

    unsafe {
        INITRAMFS_ADDR = module.addr() as usize;
        INITRAMFS_SIZE = module.size() as usize;
    }
    vfs::init(unsafe { INITRAMFS_ADDR }, unsafe { INITRAMFS_SIZE });

    let entry_addr = load_init_entry().expect("failed to stage init image");
    let root_pid = PROCESS_STACK
        .lock()
        .push_initial(entry_addr)
        .expect("create root process");

    let _ = writeln!(
        TTY.lock(),
        "[kernel] process stack ready: root pid={}",
        root_pid
    );
    let _ = writeln!(TTY.lock(), "[kernel] launching init @ {:#x}", entry_addr);

    let entry: extern "C" fn() -> ! = unsafe { core::mem::transmute(entry_addr) };
    entry()
}

fn load_named_entry(path: &str) -> Option<usize> {
    let archive =
        unsafe { core::slice::from_raw_parts(INITRAMFS_ADDR as *const u8, INITRAMFS_SIZE) };

    let cleaned = path.trim_start_matches('/');
    let file = find_file(archive, cleaned).or_else(|| {
        let basename = cleaned.rsplit('/').next()?;
        find_file(archive, basename)
    })?;

    let image = parse_elf64(file.data)?;
    elf_loader::load_init_image(file.data, &image.program_headers, image.entry)
}

fn load_init_entry() -> Option<usize> {
    load_named_entry("init.elf")
}

#[unsafe(no_mangle)]
extern "C" fn syscall_dispatch(nr: u64, fd: u64, ptr: u64, len: u64) -> i64 {
    match nr {
        SYS_OPEN => {
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, fd as usize) };
            let Ok(path) = core::str::from_utf8(bytes) else {
                return -22;
            };
            let Some(handle) = vfs::open(path) else {
                return -2;
            };
            let mut stack = PROCESS_STACK.lock();
            let Some(proc) = stack.current_mut() else {
                return -3;
            };
            proc.install_fd(handle).map(|n| n as i64).unwrap_or(-24)
        }
        SYS_WRITE => {
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let mut stack = PROCESS_STACK.lock();
            let Some(proc) = stack.current_mut() else {
                return -3;
            };
            let Some((handle, _)) = proc.resolve_fd(fd) else {
                return -9;
            };
            match vfs::write(handle, bytes) {
                Ok(n) => n as i64,
                Err(e) => e,
            }
        }
        SYS_READ => {
            let mut stack = PROCESS_STACK.lock();
            let Some(proc) = stack.current_mut() else {
                return -3;
            };
            let Some((handle, offset)) = proc.resolve_fd(fd) else {
                return -9;
            };
            let dst = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len as usize) };
            match vfs::read(handle, offset, dst) {
                Ok(n) => {
                    let _ = proc.advance_fd(fd, n);
                    n as i64
                }
                Err(e) => e,
            }
        }
        SYS_MEMMAP => {
            let req_len = fd as usize;
            memory::MEM_MANAGER
                .lock()
                .memmap(req_len)
                .map(|addr| addr as i64)
                .unwrap_or(-12)
        }
        SYS_FORK => {
            let mut stack = PROCESS_STACK.lock();
            match stack.fork_current((ptr != 0).then_some(ptr as usize)) {
                Ok(child_pid) => {
                    let _ = writeln!(
                        TTY.lock(),
                        "[kernel] fork: pushed child pid={} (running child first)",
                        child_pid
                    );
                    0
                }
                Err(_) => -12,
            }
        }
        SYS_EXECVE => {
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, fd as usize) };
            let Ok(path) = core::str::from_utf8(bytes) else {
                return -22;
            };

            let Some(entry_addr) = load_named_entry(path) else {
                return -2;
            };
            if PROCESS_STACK.lock().exec_current(entry_addr).is_err() {
                return -1;
            }
            let _ = writeln!(
                TTY.lock(),
                "[kernel] execve: replaced current process image with {}",
                path
            );
            let entry: extern "C" fn() -> ! = unsafe { core::mem::transmute(entry_addr) };
            entry()
        }
        SYS_EXIT => {
            let code = fd as i32;
            let mut stack = PROCESS_STACK.lock();
            let next = stack.exit_current().ok().flatten();
            if let Some(pid) = next {
                let resume_rip = stack.current().map(|p| p.context.rip).unwrap_or(0);
                let _ = writeln!(
                    TTY.lock(),
                    "[kernel] exit({}): popped current process, now pid={} on top",
                    code,
                    pid
                );
                drop(stack);
                if resume_rip != 0 {
                    let entry: extern "C" fn() -> ! = unsafe { core::mem::transmute(resume_rip) };
                    entry()
                }
                pid as i64
            } else {
                let _ = writeln!(TTY.lock(), "[kernel] exit({}): process stack empty", code);
                unsafe {
                    asm!("out dx, al", in("dx") 0xF4u16, in("al") 0x10u8, options(nostack, nomem));
                }
                0
            }
        }
        _ => -38,
    }
}
