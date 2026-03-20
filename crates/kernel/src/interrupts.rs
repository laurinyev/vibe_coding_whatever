use core::arch::{asm, global_asm};

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

unsafe extern "C" {
    fn syscall_int80();
}

pub fn install_idt() {
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
