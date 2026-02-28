#[derive(Clone, Copy, Debug)]
pub struct ProgramHeader {
    pub file_offset: usize,
    pub virt_addr: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ElfImage<'a> {
    pub entry: usize,
    pub data: &'a [u8],
    pub program_headers: [Option<ProgramHeader>; 8],
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

pub fn parse_elf64(image: &[u8]) -> Option<ElfImage<'_>> {
    if image.get(0..4)? != b"\x7fELF" {
        return None;
    }
    if *image.get(4)? != 2 || *image.get(5)? != 1 {
        return None;
    }

    let entry = rd64(image, 24)? as usize;
    let phoff = rd64(image, 32)? as usize;
    let phentsize = rd16(image, 54)? as usize;
    let phnum = rd16(image, 56)? as usize;

    let mut headers = [None; 8];
    let mut seen = 0usize;

    for i in 0..phnum {
        let o = phoff + i * phentsize;
        let p_type = rd32(image, o)?;
        if p_type != 1 {
            continue;
        }
        if seen == headers.len() {
            break;
        }
        headers[seen] = Some(ProgramHeader {
            flags: rd32(image, o + 4)?,
            file_offset: rd64(image, o + 8)? as usize,
            virt_addr: rd64(image, o + 16)? as usize,
            file_size: rd64(image, o + 32)? as usize,
            mem_size: rd64(image, o + 40)? as usize,
        });
        seen += 1;
    }

    Some(ElfImage {
        entry,
        data: image,
        program_headers: headers,
    })
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::parse_elf64;

    #[test]
    fn rejects_non_elf() {
        assert!(parse_elf64(b"nope").is_none());
    }
}
