#![no_std]
#![no_main]

mod elf_loader;
mod interrupts;
mod memory;
mod serial;
mod tty;

use common::elf::parse_elf64;
use common::syscall::{FD_STDIN, FD_STDOUT, SYS_MEMMAP, SYS_READ, SYS_WRITE};
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

static INPUT: Mutex<[u8; 128]> = Mutex::new([0; 128]);

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

    let archive = unsafe { core::slice::from_raw_parts(module.addr(), module.size() as usize) };

    let init_elf = find_file(archive, "init.elf").expect("init.elf missing");
    let image = parse_elf64(init_elf.data).expect("bad init.elf");
    let entry_addr =
        elf_loader::load_init_image(init_elf.data, &image.program_headers, image.entry)
            .expect("failed to stage init image");

    let _ = writeln!(TTY.lock(), "[kernel] launching init @ {:#x}", entry_addr);

    let entry: extern "C" fn() -> ! = unsafe { core::mem::transmute(entry_addr) };
    entry()
}

#[unsafe(no_mangle)]
extern "C" fn syscall_dispatch(nr: u64, fd: u64, ptr: u64, len: u64) -> i64 {
    match nr {
        SYS_WRITE if fd == FD_STDOUT => {
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
            if let Ok(s) = core::str::from_utf8(bytes) {
                let _ = write!(TTY.lock(), "{s}");
                len as i64
            } else {
                -22
            }
        }
        SYS_MEMMAP => {
            let len = fd as usize;
            memory::MEM_MANAGER
                .lock()
                .memmap(len)
                .map(|addr| addr as i64)
                .unwrap_or(-12)
        }
        SYS_READ if fd == FD_STDIN => {
            let mut input = INPUT.lock();
            let canned = b"typed-from-kernel\n";
            input[..canned.len()].copy_from_slice(canned);
            let n = core::cmp::min(len as usize, canned.len());
            let dst = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, n) };
            dst.copy_from_slice(&input[..n]);
            n as i64
        }
        _ => -38,
    }
}
