use std::collections::VecDeque;
use std::io::{self, Write};

type Pid = u32;

#[derive(Debug, Clone)]
enum ProcessState {
    Ready,
    Running,
    Sleeping,
    Terminated,
}

#[derive(Debug, Clone)]
struct Process {
    pid: Pid,
    name: String,
    state: ProcessState,
    ticks: u64,
}

struct TinyOs {
    next_pid: Pid,
    processes: Vec<Process>,
    scheduler_queue: VecDeque<Pid>,
    total_memory_mb: u32,
    used_memory_mb: u32,
}

impl TinyOs {
    fn boot() -> Self {
        let mut os = Self {
            next_pid: 1,
            processes: Vec::new(),
            scheduler_queue: VecDeque::new(),
            total_memory_mb: 256,
            used_memory_mb: 20,
        };

        os.spawn("init");
        os.spawn("shell");

        println!("TinyOS booted successfully!");
        println!("Type 'help' to see available commands.");

        os
    }

    fn spawn(&mut self, name: &str) {
        let pid = self.next_pid;
        self.next_pid += 1;

        self.processes.push(Process {
            pid,
            name: name.to_string(),
            state: ProcessState::Ready,
            ticks: 0,
        });

        self.scheduler_queue.push_back(pid);
        self.used_memory_mb = (self.used_memory_mb + 4).min(self.total_memory_mb);

        println!("[kernel] spawned process '{name}' with pid {pid}");
    }

    fn list_processes(&self) {
        println!("PID\tSTATE\t\tTICKS\tNAME");
        for process in &self.processes {
            let state = match process.state {
                ProcessState::Ready => "Ready",
                ProcessState::Running => "Running",
                ProcessState::Sleeping => "Sleeping",
                ProcessState::Terminated => "Terminated",
            };
            println!(
                "{}\t{}\t\t{}\t{}",
                process.pid, state, process.ticks, process.name
            );
        }
    }

    fn kill(&mut self, pid: Pid) {
        if let Some(process) = self.processes.iter_mut().find(|p| p.pid == pid) {
            process.state = ProcessState::Terminated;
            println!("[kernel] terminated pid {pid}");
            self.used_memory_mb = self.used_memory_mb.saturating_sub(2);
            self.scheduler_queue.retain(|queued_pid| *queued_pid != pid);
        } else {
            println!("[kernel] pid {pid} not found");
        }
    }

    fn schedule_tick(&mut self) {
        for process in &mut self.processes {
            if matches!(process.state, ProcessState::Running) {
                process.state = ProcessState::Ready;
            }
        }

        if let Some(pid) = self.scheduler_queue.pop_front() {
            if let Some(process) = self
                .processes
                .iter_mut()
                .find(|p| p.pid == pid && !matches!(p.state, ProcessState::Terminated))
            {
                if process.ticks % 5 == 4 {
                    process.state = ProcessState::Sleeping;
                    process.ticks += 1;
                    println!("[scheduler] pid {} is sleeping this tick", process.pid);
                    process.state = ProcessState::Ready;
                } else {
                    process.state = ProcessState::Running;
                    process.ticks += 1;
                    println!("[scheduler] running pid {} ({})", process.pid, process.name);
                }

                self.scheduler_queue.push_back(pid);
            }
        } else {
            println!("[scheduler] no runnable processes");
        }
    }

    fn memory(&self) {
        let free = self.total_memory_mb.saturating_sub(self.used_memory_mb);
        println!(
            "Memory: {} MB used / {} MB total ({} MB free)",
            self.used_memory_mb, self.total_memory_mb, free
        );
    }

    fn help() {
        println!("Commands:");
        println!("  help                - Show this menu");
        println!("  ps                  - List running processes");
        println!("  spawn <name>        - Spawn a process");
        println!("  kill <pid>          - Terminate a process");
        println!("  tick                - Run one scheduler tick");
        println!("  mem                 - Show memory usage");
        println!("  uptime              - Show total scheduler ticks");
        println!("  exit                - Shutdown TinyOS");
    }

    fn uptime(&self) {
        let total_ticks: u64 = self.processes.iter().map(|p| p.ticks).sum();
        println!("Uptime ticks: {total_ticks}");
    }
}

fn main() {
    let mut os = TinyOs::boot();
    let stdin = io::stdin();

    loop {
        print!("tinyos> ");
        if io::stdout().flush().is_err() {
            eprintln!("[kernel panic] failed to flush stdout");
            break;
        }

        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            eprintln!("[kernel panic] failed to read command");
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let mut parts = input.split_whitespace();
        let command = parts.next().unwrap_or_default();

        match command {
            "help" => TinyOs::help(),
            "ps" => os.list_processes(),
            "spawn" => {
                if let Some(name) = parts.next() {
                    os.spawn(name);
                } else {
                    println!("usage: spawn <name>");
                }
            }
            "kill" => {
                if let Some(pid_raw) = parts.next() {
                    match pid_raw.parse::<Pid>() {
                        Ok(pid) => os.kill(pid),
                        Err(_) => println!("invalid pid: {pid_raw}"),
                    }
                } else {
                    println!("usage: kill <pid>");
                }
            }
            "tick" => os.schedule_tick(),
            "mem" => os.memory(),
            "uptime" => os.uptime(),
            "exit" => {
                println!("Shutting down TinyOS... bye.");
                break;
            }
            _ => println!("unknown command: {input} (try 'help')"),
        }
    }
}
