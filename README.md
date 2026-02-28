# TinyOS (Rust)

I made this because I was **bored to shit** and wanted to try **vibecoding**.

## What this is

This is a tiny operating-system-style simulator written in Rust. It is not a real bootable kernel, but it mimics a few core OS concepts:

- Process table
- Round-robin scheduler ticks
- Process creation (`spawn`) and termination (`kill`)
- Basic memory usage simulation
- Tiny shell loop for commands

## Run

```bash
cargo run
```

## Commands

- `help`
- `ps`
- `spawn <name>`
- `kill <pid>`
- `tick`
- `mem`
- `uptime`
- `exit`
