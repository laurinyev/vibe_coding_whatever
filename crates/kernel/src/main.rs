#![no_std]
#![no_main]

use common::elf::{ProgramHeader, parse_elf64};
use common::syscall::{FD_STDIN, FD_STDOUT, SYS_MEMMAP, SYS_READ, SYS_WRITE};
use common::ustar::find_file;
use core::arch::{asm, global_asm};
use core::fmt::{self, Write};
use core::ptr;
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
.global syscall_int80
syscall_int80:
    push rcx
    push rdi
    push rsi
    push rdx
    mov rcx, rdx
    mov rdx, rsi
    mov rsi, rdi
    mov rdi, rax
    call syscall_dispatch
    pop rdx
    pop rsi
    pop rdi
    pop rcx
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
static MEM_MANAGER: Mutex<MemManager> = Mutex::new(MemManager::new());

const INIT_LOAD_BUF_SIZE: usize = 2 * 1024 * 1024;
static mut INIT_LOAD_BUF: [u8; INIT_LOAD_BUF_SIZE] = [0; INIT_LOAD_BUF_SIZE];

const USER_MEM_POOL_SIZE: usize = 1024 * 1024;
static mut USER_MEM_POOL: [u8; USER_MEM_POOL_SIZE] = [0; USER_MEM_POOL_SIZE];

struct MemManager {
    next: usize,
}

impl MemManager {
    const fn new() -> Self {
        Self { next: 0 }
    }

    fn memmap(&mut self, length: usize) -> Option<usize> {
        let aligned = (length + 0xfff) & !0xfff;
        let off = (self.next + 0xfff) & !0xfff;
        if off.checked_add(aligned)? > USER_MEM_POOL_SIZE {
            return None;
        }
        self.next = off + aligned;
        let base = core::ptr::addr_of_mut!(USER_MEM_POOL) as *mut u8;
        Some(base as usize + off)
    }
}

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
    let _ = writeln!(
        TTY.lock(),
        "[kernel] simple mem manager ready ({} KiB)",
        USER_MEM_POOL_SIZE / 1024
    );

    install_idt();

    let module = MODULE_REQUEST
        .get_response()
        .and_then(|m| m.modules().first().copied())
        .expect("missing initramfs module");

    let archive = unsafe { core::slice::from_raw_parts(module.addr(), module.size() as usize) };

    let init_elf = find_file(archive, "init.elf").expect("init.elf missing");
    let image = parse_elf64(init_elf.data).expect("bad init.elf");
    let entry_addr = load_init_image(init_elf.data, &image.program_headers, image.entry)
        .expect("failed to stage init image");

    let _ = writeln!(TTY.lock(), "[kernel] launching init @ {:#x}", entry_addr);

    let entry: extern "C" fn() -> ! = unsafe { core::mem::transmute(entry_addr) };
    entry()
}

fn rd16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(o)?, *b.get(o + 1)?]))
}

fn rd32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(o)?,
        *b.get(o + 1)?,
        *b.get(o + 2)?,
        *b.get(o + 3)?,
    ]))
}

fn rd64(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes([
        *b.get(o)?,
        *b.get(o + 1)?,
        *b.get(o + 2)?,
        *b.get(o + 3)?,
        *b.get(o + 4)?,
        *b.get(o + 5)?,
        *b.get(o + 6)?,
        *b.get(o + 7)?,
    ]))
}

fn load_init_image(
    bytes: &[u8],
    headers: &[Option<ProgramHeader>; 8],
    entry: usize,
) -> Option<usize> {
    let mut min_vaddr = usize::MAX;
    let mut max_vaddr = 0usize;

    for hdr in headers.iter().flatten() {
        min_vaddr = min_vaddr.min(hdr.virt_addr);
        max_vaddr = max_vaddr.max(hdr.virt_addr.checked_add(hdr.mem_size)?);
    }

    if min_vaddr == usize::MAX || max_vaddr <= min_vaddr {
        return None;
    }

    let image_size = max_vaddr - min_vaddr;
    if image_size > INIT_LOAD_BUF_SIZE {
        return None;
    }

    unsafe {
        ptr::write_bytes(
            core::ptr::addr_of_mut!(INIT_LOAD_BUF) as *mut u8,
            0,
            image_size,
        )
    };

    let base = core::ptr::addr_of_mut!(INIT_LOAD_BUF) as *mut u8;

    for hdr in headers.iter().flatten() {
        let src_end = hdr.file_offset.checked_add(hdr.file_size)?;
        if src_end > bytes.len() {
            return None;
        }

        let dst_off = hdr.virt_addr.checked_sub(min_vaddr)?;
        if dst_off.checked_add(hdr.mem_size)? > image_size {
            return None;
        }

        let src = unsafe { bytes.as_ptr().add(hdr.file_offset) };
        unsafe {
            ptr::copy_nonoverlapping(src, base.add(dst_off), hdr.file_size);
            if hdr.mem_size > hdr.file_size {
                ptr::write_bytes(
                    base.add(dst_off + hdr.file_size),
                    0,
                    hdr.mem_size - hdr.file_size,
                );
            }
        }
    }

    let e_type = rd16(bytes, 16)?;
    if e_type == 3 {
        apply_relative_relocations(bytes, base as usize, min_vaddr)?;
    }

    entry
        .checked_sub(min_vaddr)
        .map(|entry_off| base as usize + entry_off)
}

fn apply_relative_relocations(bytes: &[u8], base: usize, min_vaddr: usize) -> Option<()> {
    let phoff = rd64(bytes, 32)? as usize;
    let phentsize = rd16(bytes, 54)? as usize;
    let phnum = rd16(bytes, 56)? as usize;

    let mut rela_vaddr = 0usize;
    let mut rela_size = 0usize;
    let mut rela_ent = 24usize;

    for i in 0..phnum {
        let o = phoff + i * phentsize;
        if rd32(bytes, o)? != 2 {
            continue;
        }
        let dyn_off = rd64(bytes, o + 8)? as usize;
        let dyn_size = rd64(bytes, o + 32)? as usize;
        let end = dyn_off.checked_add(dyn_size)?;
        if end > bytes.len() {
            return None;
        }

        let mut d = dyn_off;
        while d + 16 <= end {
            let tag = rd64(bytes, d)? as i64;
            let val = rd64(bytes, d + 8)? as usize;
            match tag {
                0 => break,
                7 => rela_vaddr = val,
                8 => rela_size = val,
                9 => rela_ent = val,
                _ => {}
            }
            d += 16;
        }
    }

    if rela_vaddr == 0 || rela_size == 0 || rela_ent == 0 {
        return Some(());
    }

    let rela_off = rela_vaddr.checked_sub(min_vaddr)?;
    let mut off = base.checked_add(rela_off)?;
    let end = off.checked_add(rela_size)?;

    while off < end {
        let r_offset = unsafe { *(off as *const u64) } as usize;
        let r_info = unsafe { *((off + 8) as *const u64) };
        let r_addend = unsafe { *((off + 16) as *const i64) } as isize;

        let r_type = (r_info & 0xffff_ffff) as u32;
        if r_type == 8 {
            let dst = base.checked_add(r_offset.checked_sub(min_vaddr)?)? as *mut u64;
            let val = (base as isize + r_addend) as u64;
            unsafe { *dst = val };
        }

        off = off.checked_add(rela_ent)?;
    }

    Some(())
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

    fn set(&mut self, addr: u64, dpl: u8, selector: u16) {
        self.off1 = addr as u16;
        self.sel = selector;
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
        let cs: u16;
        asm!("mov {0:x}, cs", out(reg) cs, options(nostack, preserves_flags));
        IDT[0x80].set(syscall_int80 as usize as u64, 3, cs);
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
        SYS_MEMMAP => {
            let len = fd as usize;
            MEM_MANAGER
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

const COM1: u16 = 0x3F8;

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
    unsafe { outb(COM1, byte) }
}

unsafe fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, nomem)) }
}
