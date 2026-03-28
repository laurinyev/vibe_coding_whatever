use crate::serial::serial_write_byte;
use core::fmt::{self, Write};
use spin::Mutex;

pub struct Tty {
    fb: Option<limine::framebuffer::Framebuffer<'static>>,
    x: usize,
    y: usize,
}

#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bytes_per_pixel: usize,
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

pub fn write_bytes(bytes: &[u8]) {
    let mut tty = TTY.lock();
    for &b in bytes {
        tty.putc(b);
    }
}

pub fn framebuffer_info() -> Option<FramebufferInfo> {
    let tty = TTY.lock();
    let fb = tty.fb.as_ref()?;
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pitch = fb.pitch() as usize;
    let bytes_per_pixel = (fb.bpp() / 8) as usize;
    Some(FramebufferInfo {
        width,
        height,
        pitch,
        bytes_per_pixel,
    })
}

pub fn framebuffer_read(offset: usize, dst: &mut [u8]) -> usize {
    let tty = TTY.lock();
    let Some(fb) = tty.fb.as_ref() else {
        return 0;
    };
    let info_height = fb.height() as usize;
    let size = (fb.pitch() as usize).saturating_mul(info_height);
    if offset >= size {
        return 0;
    }
    let n = core::cmp::min(size - offset, dst.len());
    let src = unsafe { core::slice::from_raw_parts(fb.addr().add(offset) as *const u8, n) };
    dst[..n].copy_from_slice(src);
    n
}

pub fn framebuffer_write(bytes: &[u8]) -> usize {
    let mut tty = TTY.lock();
    let Some(fb) = tty.fb.as_mut() else {
        return 0;
    };
    let info_height = fb.height() as usize;
    let size = (fb.pitch() as usize).saturating_mul(info_height);
    let n = core::cmp::min(size, bytes.len());
    let dst = unsafe { core::slice::from_raw_parts_mut(fb.addr() as *mut u8, n) };
    dst.copy_from_slice(&bytes[..n]);
    n
}
