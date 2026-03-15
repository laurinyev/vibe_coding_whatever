#![allow(clippy::module_name_repetitions)]

pub const PROCESS_FD_CAPACITY: usize = 8;
pub const FD_NONE: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessContext {
    pub rip: usize,
    pub rsp: usize,
    pub rax: usize,
}

impl ProcessContext {
    pub const fn new(rip: usize, rsp: usize) -> Self {
        Self { rip, rsp, rax: 0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u64,
    pub pagemap: usize,
    pub fds: [u64; PROCESS_FD_CAPACITY],
    pub fd_offsets: [usize; PROCESS_FD_CAPACITY],
    pub context: ProcessContext,
}

impl Process {
    pub const fn new(pid: u64, entry: usize) -> Self {
        let mut fds = [FD_NONE; PROCESS_FD_CAPACITY];
        fds[0] = 0;
        fds[1] = 1;
        fds[2] = 2;
        Self {
            pid,
            pagemap: pid as usize * 0x1000,
            fds,
            fd_offsets: [0; PROCESS_FD_CAPACITY],
            context: ProcessContext::new(entry, 0),
        }
    }

    pub fn resolve_fd(&self, fd: u64) -> Option<(u64, usize)> {
        let idx = usize::try_from(fd).ok()?;
        let handle = *self.fds.get(idx)?;
        if handle == FD_NONE {
            return None;
        }
        Some((handle, self.fd_offsets[idx]))
    }

    pub fn advance_fd(&mut self, fd: u64, amount: usize) -> Option<()> {
        let idx = usize::try_from(fd).ok()?;
        let off = self.fd_offsets.get_mut(idx)?;
        *off = off.checked_add(amount)?;
        Some(())
    }

    pub fn install_fd(&mut self, handle: u64) -> Option<u64> {
        for i in 3..PROCESS_FD_CAPACITY {
            if self.fds[i] == FD_NONE {
                self.fds[i] = handle;
                self.fd_offsets[i] = 0;
                return Some(i as u64);
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessError {
    StackFull,
    StackEmpty,
}

pub struct ProcessStack<const N: usize> {
    stack: [Option<Process>; N],
    depth: usize,
    next_pid: u64,
}

impl<const N: usize> ProcessStack<N> {
    pub const fn new() -> Self {
        Self {
            stack: [None; N],
            depth: 0,
            next_pid: 1,
        }
    }

    pub fn push_initial(&mut self, entry: usize) -> Result<u64, ProcessError> {
        if self.depth != 0 {
            return Ok(self.stack[0].map_or(1, |p| p.pid));
        }
        let pid = self.alloc_pid();
        self.push(Process::new(pid, entry))?;
        Ok(pid)
    }

    pub fn current(&self) -> Option<Process> {
        self.depth.checked_sub(1).and_then(|i| self.stack[i])
    }

    pub fn fork_current(&mut self, parent_resume_rip: Option<usize>) -> Result<u64, ProcessError> {
        let mut child = self.current().ok_or(ProcessError::StackEmpty)?;
        let child_pid = self.alloc_pid();
        child.pid = child_pid;
        child.pagemap = child_pid as usize * 0x1000;
        child.context.rax = 0;
        if let Some(parent) = self.current_mut() {
            parent.context.rax = child_pid as usize;
            if let Some(rip) = parent_resume_rip {
                parent.context.rip = rip;
            }
        }
        self.push(child)?;
        Ok(child_pid)
    }

    pub fn exec_current(&mut self, entry: usize) -> Result<(), ProcessError> {
        let proc = self.current_mut().ok_or(ProcessError::StackEmpty)?;
        proc.context = ProcessContext::new(entry, 0);
        proc.pagemap = proc.pid as usize * 0x2000;
        Ok(())
    }

    pub fn exit_current(&mut self) -> Result<Option<u64>, ProcessError> {
        if self.depth == 0 {
            return Err(ProcessError::StackEmpty);
        }
        self.depth -= 1;
        self.stack[self.depth] = None;
        Ok(self.current().map(|p| p.pid))
    }

    fn push(&mut self, p: Process) -> Result<(), ProcessError> {
        if self.depth >= N {
            return Err(ProcessError::StackFull);
        }
        self.stack[self.depth] = Some(p);
        self.depth += 1;
        Ok(())
    }

    fn alloc_pid(&mut self) -> u64 {
        let pid = self.next_pid;
        self.next_pid += 1;
        pid
    }

    pub fn current_mut(&mut self) -> Option<&mut Process> {
        self.depth
            .checked_sub(1)
            .and_then(move |i| self.stack[i].as_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::ProcessStack;

    #[test]
    fn fork_pushes_new_top_and_exit_restores_previous() {
        let mut stack: ProcessStack<8> = ProcessStack::new();
        let pid1 = stack.push_initial(0x1000).expect("initial pid");
        assert_eq!(pid1, 1);

        let child = stack.fork_current(None).expect("fork");
        assert_eq!(child, 2);
        assert_eq!(stack.current().expect("current").pid, child);

        let underneath = stack.exit_current().expect("exit").expect("underneath");
        assert_eq!(underneath, pid1);
        assert_eq!(stack.current().expect("current").pid, pid1);
    }

    #[test]
    fn exec_replaces_context() {
        let mut stack: ProcessStack<4> = ProcessStack::new();
        stack.push_initial(0x1111).expect("initial");
        stack.exec_current(0x2222).expect("exec");
        assert_eq!(stack.current().expect("proc").context.rip, 0x2222);
    }

    #[test]
    fn install_and_resolve_fd() {
        let mut stack: ProcessStack<2> = ProcessStack::new();
        stack.push_initial(0x1000).expect("initial");
        let proc = stack.current_mut().expect("proc");
        let fd = proc.install_fd(123).expect("fd");
        assert_eq!(proc.resolve_fd(fd), Some((123, 0)));
    }
}
