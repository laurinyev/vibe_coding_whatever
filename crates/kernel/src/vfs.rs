use crate::serial::{serial_read_byte_blocking, serial_try_read_byte, serial_write_byte};
use crate::tty::{framebuffer_info, framebuffer_read, framebuffer_write, write_bytes};
use common::ustar::find_file;
use spin::Mutex;

const HANDLE_STDIN: u64 = 0;
const HANDLE_STDOUT: u64 = 1;
const HANDLE_STDERR: u64 = 2;
const HANDLE_FB0: u64 = 3;
const HANDLE_BASE_INITRD: u64 = 4;
const MAX_OPEN_FILES: usize = 32;

#[derive(Clone, Copy)]
enum Node {
    DevStdin,
    DevStdout,
    DevStderr,
    DevFramebuffer,
    Initrd { data_addr: usize, len: usize },
}

struct VfsState {
    initrd_addr: usize,
    initrd_size: usize,
    nodes: [Option<Node>; MAX_OPEN_FILES],
}

impl VfsState {
    const fn new() -> Self {
        let mut nodes = [None; MAX_OPEN_FILES];
        nodes[0] = Some(Node::DevStdin);
        nodes[1] = Some(Node::DevStdout);
        nodes[2] = Some(Node::DevStderr);
        nodes[3] = Some(Node::DevFramebuffer);
        Self {
            initrd_addr: 0,
            initrd_size: 0,
            nodes,
        }
    }
}

static VFS: Mutex<VfsState> = Mutex::new(VfsState::new());

pub fn init(initrd_addr: usize, initrd_size: usize) {
    let mut vfs = VFS.lock();
    vfs.initrd_addr = initrd_addr;
    vfs.initrd_size = initrd_size;
}

pub fn open(path: &str) -> Option<u64> {
    if path == "/dev/stdin" || path == "dev/stdin" {
        return Some(HANDLE_STDIN);
    }
    if path == "/dev/stdout" || path == "dev/stdout" {
        return Some(HANDLE_STDOUT);
    }
    if path == "/dev/stderr" || path == "dev/stderr" {
        return Some(HANDLE_STDERR);
    }
    if path == "/dev/fb0" || path == "dev/fb0" {
        return Some(HANDLE_FB0);
    }

    let clean = path.trim_start_matches('/');
    let mut vfs = VFS.lock();
    let archive =
        unsafe { core::slice::from_raw_parts(vfs.initrd_addr as *const u8, vfs.initrd_size) };
    let file = find_file(archive, clean)?;

    for i in HANDLE_BASE_INITRD as usize..MAX_OPEN_FILES {
        if vfs.nodes[i].is_none() {
            vfs.nodes[i] = Some(Node::Initrd {
                data_addr: file.data.as_ptr() as usize,
                len: file.data.len(),
            });
            return Some(i as u64);
        }
    }
    None
}

pub fn read(handle: u64, offset: usize, dst: &mut [u8]) -> Result<usize, i64> {
    let node = node_for(handle).ok_or(-9)?;
    match node {
        Node::DevStdin => {
            if dst.is_empty() {
                return Ok(0);
            }

            let mut n = 0;
            while n < dst.len() {
                let mut b = if n == 0 {
                    serial_read_byte_blocking()
                } else {
                    let Some(byte) = serial_try_read_byte() else {
                        break;
                    };
                    byte
                };

                if b == b'\r' {
                    b = b'\n';
                }

                dst[n] = b;
                n += 1;
                serial_write_byte(b);
                if b == b'\n' {
                    break;
                }
            }
            Ok(n)
        }
        Node::Initrd { data_addr, len } => {
            if offset >= len {
                return Ok(0);
            }
            let available = len - offset;
            let n = core::cmp::min(available, dst.len());
            let src = unsafe { core::slice::from_raw_parts((data_addr + offset) as *const u8, n) };
            dst[..n].copy_from_slice(src);
            Ok(n)
        }
        Node::DevFramebuffer => {
            if offset == 0 && dst.len() >= 32 {
                if let Some(info) = framebuffer_info() {
                    dst[..8].copy_from_slice(&(info.width as u64).to_le_bytes());
                    dst[8..16].copy_from_slice(&(info.height as u64).to_le_bytes());
                    dst[16..24].copy_from_slice(&(info.pitch as u64).to_le_bytes());
                    dst[24..32].copy_from_slice(&(info.bytes_per_pixel as u64).to_le_bytes());
                    return Ok(32);
                }
            }
            Ok(framebuffer_read(offset, dst))
        }
        Node::DevStdout | Node::DevStderr => Err(-9),
    }
}

pub fn write(handle: u64, bytes: &[u8]) -> Result<usize, i64> {
    let node = node_for(handle).ok_or(-9)?;
    match node {
        Node::DevStdout | Node::DevStderr => {
            write_bytes(bytes);
            Ok(bytes.len())
        }
        Node::DevFramebuffer => Ok(framebuffer_write(bytes)),
        Node::DevStdin | Node::Initrd { .. } => Err(-9),
    }
}

fn node_for(handle: u64) -> Option<Node> {
    let idx = usize::try_from(handle).ok()?;
    let vfs = VFS.lock();
    vfs.nodes.get(idx).copied().flatten()
}
