#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UstarEntry<'a> {
    pub name: &'a str,
    pub data: &'a [u8],
}

fn parse_octal(field: &[u8]) -> Option<usize> {
    let mut value = 0usize;
    let mut found = false;
    for &b in field {
        if b == 0 || b == b' ' {
            continue;
        }
        if !(b'0'..=b'7').contains(&b) {
            break;
        }
        found = true;
        value = (value << 3) + (b - b'0') as usize;
    }
    found.then_some(value)
}

pub fn find_file<'a>(archive: &'a [u8], needle: &str) -> Option<UstarEntry<'a>> {
    let mut off = 0usize;
    while off + 512 <= archive.len() {
        let header = &archive[off..off + 512];
        if header.iter().all(|&b| b == 0) {
            return None;
        }

        let name_end = header[0..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = core::str::from_utf8(&header[0..name_end]).ok()?;
        let size = parse_octal(&header[124..136])?;

        let data_start = off + 512;
        let data_end = data_start.checked_add(size)?;
        if data_end > archive.len() {
            return None;
        }

        if name == needle {
            return Some(UstarEntry {
                name,
                data: &archive[data_start..data_end],
            });
        }

        let padded = (size + 511) & !511;
        off = data_start + padded;
    }
    None
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::find_file;

    fn mk_header(name: &str, payload: &[u8]) -> [u8; 512] {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[100..108].copy_from_slice(b"0000644\0");
        h[108..116].copy_from_slice(b"0000000\0");
        h[116..124].copy_from_slice(b"0000000\0");
        let size = std::format!("{:011o}\0", payload.len());
        h[124..136].copy_from_slice(size.as_bytes());
        h[257..263].copy_from_slice(b"ustar\0");
        h
    }

    #[test]
    fn finds_named_file() {
        let payload = b"hello";
        let mut tar = std::vec::Vec::new();
        tar.extend_from_slice(&mk_header("init.elf", payload));
        tar.extend_from_slice(payload);
        tar.resize(1024, 0);

        let e = find_file(&tar, "init.elf").expect("file");
        assert_eq!(e.data, payload);
    }
}
