# Porthouse

A lighthouse for your ports. Monitors listening ports, detects conflicts, and helps you manage port assignments across multiple projects.

## The Problem

You're running 5 projects locally — each with a frontend, backend, and database. Port 3000 is taken. Port 8000 is taken. Postgres is fighting Docker. You run `lsof -i :3000` for the 50th time today.

Porthouse fixes this.

## Why Porthouse

- **One command to see everything** — `porthouse status` replaces `lsof -i | grep LISTEN | sort | ...`
- **Real conflict detection** — only flags actual conflicts (different processes on the same port), ignores normal dual-stack bindings that flood `lsof` output
- **Project-aware** — register your projects with port ranges so you always know which project owns which port
- **Background monitoring** — daemon watches for conflicts 24/7 and sends native macOS notifications the moment something goes wrong
- **CI/CD friendly** — `porthouse check --quiet` exits with code 1 on conflicts, `--json` for machine-readable output
- **Tiny footprint** — 2 MB binary, <5 MB RAM, <0.1% CPU. No runtime dependencies, no Docker, no Node.js
- **Works on servers too** — same tool scales from your laptop to staging servers to shared dev environments

## Install

```bash
# From source (requires Rust)
cargo install --git https://github.com/mohavinash/porthouse

# Or clone and build
git clone https://github.com/mohavinash/porthouse.git
cd porthouse
cargo install --path .
```

## Quick Start

```bash
# See what's running on your ports
porthouse status

# Check for port conflicts
porthouse check

# Find 3 free ports
porthouse suggest 3

# Is port 8080 available?
porthouse free 8080

# Kill whatever is on port 3000
porthouse kill 3000
```

## Commands

### `porthouse status`

Shows all listening ports with process info:

```
PORT     PID      PROCESS              PROTO    ADDRESS
------------------------------------------------------------
3000     4357     node                 TCP      ::1
5432     65109    postgres             TCP      127.0.0.1
5432     30215    com.docker.backend   TCP      ::
8000     1783     Python               TCP      127.0.0.1
```

### `porthouse check`

Detects real conflicts — different processes fighting over the same port. Exits with code 1 if conflicts exist (useful in scripts and CI).

```bash
$ porthouse check
Found 1 conflict(s):
  Port 5432:
    - postgres (PID 65109)
    - com.docker.backend (PID 30215)

# Use in scripts
porthouse check --quiet || echo "Fix your ports!"

# Machine-readable output
porthouse check --json
```

Smart enough to ignore dual-stack bindings (same process on `0.0.0.0` and `::`) — only flags actual conflicts between different processes.

### `porthouse suggest [COUNT]`

Find free ports:

```bash
$ porthouse suggest 3
1026
1027
1028

# Within a specific range
$ porthouse suggest 3 --from 8000 --to 9000
8001
8002
8003
```

### `porthouse free <PORT>`

Check a specific port:

```bash
$ porthouse free 3000
Port 3000 is in use
  node (PID 4357) [TCP] on ::1

$ porthouse free 9999
Port 9999 is free
```

### `porthouse kill <PORT>`

Kill the process on a port:

```bash
$ porthouse kill 3000
Killing node (PID 4357) on port 3000
```

### `porthouse register <NAME>`

Register a project with reserved ports:

```bash
# Reserve a range for your project
porthouse register my-app --range 3000-3010 --ports 3000,3001

# The registry is stored at ~/.porthouse/registry.toml
```

### `porthouse daemon`

Run a background monitor that alerts on port conflicts:

```bash
porthouse daemon start    # Start monitoring (runs in foreground, use & to background)
porthouse daemon status   # Check if daemon is running
porthouse daemon stop     # Stop the daemon
```

The daemon scans every 3 seconds and sends alerts via:
- macOS notifications
- Terminal messages
- Log file (`~/.porthouse/alerts.log`)
- Webhook (optional, for Slack/Discord)

## TUI Dashboard

Run `porthouse` with no arguments to launch the interactive dashboard:

```
porthouse
```

```
+- Porthouse - Port Monitor & Manager ----------------------------+
| +- Active Ports -----------------------------------------------+|
| | PORT   PID    PROCESS          PROJECT        STATUS          ||
| | 3000   4357   node             WA Dashboard   OK             ||
| | 5432   65109  postgres         shared         CONFLICT       ||
| | 5432   30215  com.docker       -              CONFLICT       ||
| | 8000   1783   Python           theatrelabs    OK             ||
| +--------------------------------------------------------------+|
| +- Conflicts (1) ----------------------------------------------+|
| | Port 5432: postgres vs com.docker.backend                     ||
| | Suggestion: Move to port 5433 (free)                          ||
| +--------------------------------------------------------------+|
| [q]uit  [r]efresh  [j/k]navigate  [K]ill  [s]uggest            |
+-----------------------------------------------------------------+
```

## Configuration

All config lives in `~/.porthouse/`:

**`config.toml`** — daemon and alert settings:

```toml
[daemon]
scan_interval_secs = 3
port_range = [1024, 65535]

[alerts]
macos_notifications = true
terminal_bell = true
log_file = "~/.porthouse/alerts.log"
webhook_url = ""          # optional: Slack/Discord webhook

[defaults]
ports_per_project = 10
```

**`registry.toml`** — project port reservations:

```toml
[[project]]
name = "my-frontend"
path = "/path/to/project"
ports = [3000, 3001]
range = [3000, 3010]

[[project]]
name = "shared"
ports = [5432, 6379]    # postgres, redis
```

## Claude Code Integration

Add a hook to check for conflicts before starting dev servers:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "command": "/path/to/porthouse/hooks/porthouse-check.sh"
      }
    ]
  }
}
```

The hook is silent unless a conflict is detected — zero overhead on normal commands.

## How It Works

- Uses the [`listeners`](https://docs.rs/listeners) crate for cross-platform port scanning (no `lsof` shelling)
- Conflict detection compares PIDs, not just port numbers — dual-stack bindings are not false positives
- Single static binary, no runtime dependencies
- ~2 MB binary, <5 MB memory, <0.1% CPU

## Requirements

- macOS or Linux
- Rust 1.70+ (to build from source)

## License

MIT
