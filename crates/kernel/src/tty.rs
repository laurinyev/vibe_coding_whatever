use crate::serial::serial_write_byte;
use core::fmt::{self, Write};
use spin::Mutex;

pub struct Tty {
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

pub static TTY: Mutex<Tty> = Mutex::new(Tty::new());

pub fn set_framebuffer(framebuffer: limine::framebuffer::Framebuffer<'static>) {
    TTY.lock().fb = Some(framebuffer);
}
