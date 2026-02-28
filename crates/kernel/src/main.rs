#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt::{self, Write};
use core::ptr;
use common::elf::{ProgramHeader, parse_elf64};
use common::syscall::{FD_STDIN, FD_STDOUT, SYS_READ, SYS_WRITE};
use common::ustar::find_file;
use limine::BaseRevision;
use limine::request::{FramebufferRequest, ModuleRequest, RequestsEndMarker, RequestsStartMarker};
use spin::Mutex;

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

global_asm!(
    r#"
.global _start
_start:
    call kmain
1:  hlt
    jmp 1b

.global syscall_int80
syscall_int80:
    push rdi
    push rsi
    push rdx
    mov rdi, rax
    call syscall_dispatch
    pop rdx
    pop rsi
    pop rdi
    iretq
"#
);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    let _ = writeln!(TTY.lock(), "[panic] {info}");
    loop {
        unsafe { asm!("hlt") };
    }
}

struct Tty {
    fb: Option<limine::framebuffer::Framebuffer<'static>>,
    x: usize,
    y: usize,
}

impl Tty {
    const fn new() -> Self {
        Self {
            fb: None,
            x: 8,
            y: 16,
        }
    }

    fn putc(&mut self, c: u8) {
        serial_write_byte(c);
        debugcon_write_byte(c);

        if c == b'\n' {
            self.x = 8;
            self.y += 16;
            return;
        }

        if let Some(fb) = self.fb.as_mut() {
            let pitch = fb.pitch() as usize;
            let bpp = (fb.bpp() / 8) as usize;
            let buf = fb.addr();
            let offset = self.y * pitch + self.x * bpp;
            for i in 0..bpp {
                unsafe { *buf.add(offset + i) = 0xff };
            }
        }
        self.x += 8;
    }
}

impl Write for Tty {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.putc(b);
        }
        Ok(())
    }
}

static TTY: Mutex<Tty> = Mutex::new(Tty::new());
static INPUT: Mutex<[u8; 128]> = Mutex::new([0; 128]);

unsafe extern "C" {
    fn syscall_int80();
}

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    serial_init();

    if let Some(resp) = FRAMEBUFFER_REQUEST.get_response()
        && let Some(fb) = resp.framebuffers().next()
    {
        TTY.lock().fb = Some(fb);
    }

    let _ = writeln!(TTY.lock(), "[kernel] limine boot ok");

    install_idt();

    let module = MODULE_REQUEST
        .get_response()
        .and_then(|m| m.modules().first().copied())
        .expect("missing initramfs module");

    let archive = unsafe { core::slice::from_raw_parts(module.addr(), module.size() as usize) };

    let init_elf = find_file(archive, "init.elf").expect("init.elf missing");
    let image = parse_elf64(init_elf.data).expect("bad init.elf");
    load_elf_segments(init_elf.data, &image.program_headers);

    let _ = writeln!(TTY.lock(), "[kernel] launching init @ {:#x}", image.entry);

    let entry: extern "C" fn() -> ! = unsafe { core::mem::transmute(image.entry) };
    entry()
}

fn load_elf_segments(bytes: &[u8], headers: &[Option<ProgramHeader>; 8]) {
    for hdr in headers.iter().flatten() {
        let src_end = hdr.file_offset + hdr.file_size;
        if src_end > bytes.len() {
            continue;
        }

        let dst = hdr.virt_addr as *mut u8;
        let src = unsafe { bytes.as_ptr().add(hdr.file_offset) };
        unsafe {
            ptr::copy_nonoverlapping(src, dst, hdr.file_size);
            if hdr.mem_size > hdr.file_size {
                ptr::write_bytes(dst.add(hdr.file_size), 0, hdr.mem_size - hdr.file_size);
            }
        }
    }
}

#[repr(C, packed)]
struct IdtPtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    off1: u16,
    sel: u16,
    ist: u8,
    attrs: u8,
    off2: u16,
    off3: u32,
    zero: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            off1: 0,
            sel: 0,
            ist: 0,
            attrs: 0,
            off2: 0,
            off3: 0,
            zero: 0,
        }
    }

    fn set(&mut self, addr: u64, dpl: u8) {
        self.off1 = addr as u16;
        self.sel = 0x08;
        self.ist = 0;
        self.attrs = 0x8E | ((dpl & 0x3) << 5);
        self.off2 = (addr >> 16) as u16;
        self.off3 = (addr >> 32) as u32;
        self.zero = 0;
    }
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

fn install_idt() {
    unsafe {
        IDT[0x80].set(syscall_int80 as usize as u64, 3);
        let ptr = IdtPtr {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: (&raw const IDT) as *const _ as u64,
        };
        asm!("lidt [{}]", in(reg) &ptr, options(readonly, nostack));
        asm!("sti");
    }
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

const COM1: u16 = 0x3F8;
const DEBUGCON: u16 = 0xE9;

fn serial_init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);
        outb(COM1, 0x03);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xC7);
        outb(COM1 + 4, 0x0B);
    }
}

fn serial_write_byte(byte: u8) {
    unsafe {
        while (inb(COM1 + 5) & 0x20) == 0 {}
        outb(COM1, byte);
    }
}

fn debugcon_write_byte(byte: u8) {
    unsafe { outb(DEBUGCON, byte) }
}

unsafe fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, nomem)) }
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    unsafe { asm!("in al, dx", in("dx") port, out("al") val, options(nostack, nomem)) }
    val
}
