use spin::Mutex;

pub const USER_MEM_POOL_SIZE: usize = 1024 * 1024;
static mut USER_MEM_POOL: [u8; USER_MEM_POOL_SIZE] = [0; USER_MEM_POOL_SIZE];

pub struct MemManager {
    next: usize,
}

impl MemManager {
    pub const fn new() -> Self {
        Self { next: 0 }
    }

    pub fn memmap(&mut self, length: usize) -> Option<usize> {
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

pub static MEM_MANAGER: Mutex<MemManager> = Mutex::new(MemManager::new());
