use common::elf::ProgramHeader;
use core::ptr;

const INIT_LOAD_BUF_SIZE: usize = 2 * 1024 * 1024;
static mut INIT_LOAD_BUF: [u8; INIT_LOAD_BUF_SIZE] = [0; INIT_LOAD_BUF_SIZE];

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

pub fn load_init_image(
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
