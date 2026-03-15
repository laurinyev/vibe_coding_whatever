use core::arch::asm;

const COM1: u16 = 0x3F8;

pub fn serial_init() {
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

pub fn serial_write_byte(byte: u8) {
    while unsafe { inb(COM1 + 5) } & 0x20 == 0 {}
    unsafe { outb(COM1, byte) }
}

pub fn serial_try_read_byte() -> Option<u8> {
    let ready = unsafe { inb(COM1 + 5) } & 1;
    if ready != 0 {
        Some(unsafe { inb(COM1) })
    } else {
        None
    }
}

unsafe fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, nomem)) }
}

unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe { asm!("in al, dx", out("al") val, in("dx") port, options(nostack, nomem)) };
    val
}
