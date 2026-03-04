# Porthouse

A single binary that replaces `lsof -i | grep LISTEN | sort | awk | ...` with one command. Monitors listening ports, detects real conflicts, and manages port assignments across projects.

## The Problem

Every developer running multiple projects locally hits this wall. You have 3-5 services — frontend on 3000, backend on 8000, Postgres on 5432, Redis on 6379 — and then Docker grabs 5432, a second project wants 3000, and you're debugging connectivity issues that are really port collisions.

The current toolkit for this:

| What you do now | What's wrong with it |
|---|---|
| `lsof -i :3000` | Shows raw socket entries, no conflict analysis. Dual-stack bindings (same process on `0.0.0.0` and `::`) look like conflicts but aren't. |
| `lsof -i \| grep LISTEN` | Returns 40-80 lines on a typical dev machine. No grouping, no process dedup, no actionable output. |
| `netstat -tlnp` | Linux-only, deprecated on macOS. Same raw output problem. |
| `ss -tlnp` | Linux-only. Still no conflict detection. |
| `fuser 3000/tcp` | Checks one port at a time. No overview. |

None of these tools tell you: "Port 5432 has a conflict — Postgres and Docker are both trying to bind it. Port 5433 is free."

Porthouse does.

## Measured Performance

Tested on macOS (Apple Silicon, M4 Pro):

| Metric | Measured |
|---|---|
| Binary size | **2.2 MB** |
| Full port scan | **10ms** (43 listening ports) |
| Daemon memory (RSS) | **12 MB** |
| Daemon CPU | **0.0%** (wakes every 3s, scans, sleeps) |
| Startup time | **<10ms** (no runtime, no interpreter, no JIT) |
| Dependencies at runtime | **Zero** — static binary, no Node.js, no Python, no Docker |

For comparison: a Node.js "hello world" process uses ~40 MB RSS. A Python script importing `psutil` uses ~25 MB. Porthouse's daemon monitors all your ports for less memory than a single idle Node process.

## What It Does That Existing Tools Don't

| Capability | `lsof`/`ss`/`netstat` | Port management scripts | Porthouse |
|---|---|---|---|
| List listening ports | Raw socket dump | Usually | Clean table with process names |
| Detect real conflicts | No (shows all bindings) | Sometimes | Yes — PID-aware, ignores dual-stack |
| Suggest free ports | No | Sometimes | Yes — range-aware |
| Project registry | No | No | Yes — know which project owns which port |
| Background monitoring | No | No | Yes — daemon with native alerts |
| macOS/Linux notifications | No | No | Yes |
| CI/CD integration | Manual | Manual | `--quiet` exit codes, `--json` output |
| Kill by port | `kill $(lsof -t -i:3000)` | Usually | `porthouse kill 3000` |
| TUI dashboard | No | No | Yes — real-time, keyboard-driven |

### Why PID-aware conflict detection matters

Run `lsof -i :5000` on a Mac and you'll see:

```
ControlCenter 518 TCP *:5000 (LISTEN)
ControlCenter 518 TCP [::]:5000 (LISTEN)
```

Two entries, same process (PID 518), same port. This is normal dual-stack binding — not a conflict. Every tool that just counts port occurrences will flag this as a problem.

Porthouse groups by port, then checks if there are **distinct PIDs**. Same PID on multiple addresses = normal. Different PIDs on the same port = actual conflict. This eliminates the false positives that make `lsof` output noisy and untrustworthy.

## Install

### Homebrew (macOS/Linux)

```bash
brew install mohavinash/porthouse/porthouse
```

### Scoop (Windows)

```powershell
scoop bucket add porthouse https://github.com/mohavinash/scoop-porthouse
scoop install porthouse
```

### Cargo (from source)

```bash
cargo install --git https://github.com/mohavinash/porthouse
```

### Direct Download

Prebuilt binaries for all platforms on [GitHub Releases](https://github.com/mohavinash/porthouse/releases/latest):

```bash
# macOS (Apple Silicon)
curl -L https://github.com/mohavinash/porthouse/releases/latest/download/porthouse-macos-arm64 -o porthouse
chmod +x porthouse && sudo mv porthouse /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/mohavinash/porthouse/releases/latest/download/porthouse-macos-x86_64 -o porthouse
chmod +x porthouse && sudo mv porthouse /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/mohavinash/porthouse/releases/latest/download/porthouse-linux-x86_64 -o porthouse
chmod +x porthouse && sudo mv porthouse /usr/local/bin/
```

Windows: download `porthouse-windows-x86_64.exe` from the [releases page](https://github.com/mohavinash/porthouse/releases/latest).

## Quick Start

```bash
# See what's running on your ports
porthouse status

# Check for port conflicts (exits 1 if conflicts found)
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

```
PORT     PID      PROCESS              PROTO    ADDRESS
------------------------------------------------------------
3000     4357     node                 TCP      ::1
5432     65109    postgres             TCP      127.0.0.1
5432     30215    com.docker.backend   TCP      ::
8000     1783     Python               TCP      127.0.0.1

CONFLICTS DETECTED:
------------------------------------------------------------
  Port 5432:
    - postgres (PID 65109) [TCP] on 127.0.0.1
    - com.docker.backend (PID 30215) [TCP] on ::
    Suggested alternative: port 5433
```

### `porthouse check`

```bash
$ porthouse check
Found 1 conflict(s):
  Port 5432:
    - postgres (PID 65109)
    - com.docker.backend (PID 30215)

# Silent mode for scripts — exit code only
porthouse check --quiet || echo "Fix your ports!"

# JSON for CI pipelines
porthouse check --json
```

### `porthouse suggest [COUNT]`

```bash
$ porthouse suggest 3
1026
1027
1028

$ porthouse suggest 3 --from 8000 --to 9000
8001
8002
8003
```

### `porthouse free <PORT>`

```bash
$ porthouse free 3000
Port 3000 is in use
  node (PID 4357) [TCP] on ::1

$ porthouse free 9999
Port 9999 is free
```

### `porthouse kill <PORT>`

```bash
$ porthouse kill 3000
Killing node (PID 4357) on port 3000
```

### `porthouse register <NAME>`

```bash
porthouse register my-app --range 3000-3010 --ports 3000,3001
```

The registry tracks which project owns which ports. Stored at `~/.porthouse/registry.toml`.

### `porthouse daemon`

```bash
porthouse daemon start    # Start background monitoring
porthouse daemon status   # Check if running
porthouse daemon stop     # Stop
```

Scans every 3 seconds. Alerts via macOS notifications, terminal, log file, or webhook (Slack/Discord).

## TUI Dashboard

Run `porthouse` with no arguments:

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

Real-time updates. Navigate with `j`/`k`, kill a process with `K`, refresh with `r`.

## Configuration

All config lives in `~/.porthouse/`:

**`config.toml`**

```toml
[daemon]
scan_interval_secs = 3
port_range = [1024, 49151]    # excludes ephemeral ports

[alerts]
macos_notifications = true
terminal_bell = true
log_file = "~/.porthouse/alerts.log"
webhook_url = ""              # Slack/Discord webhook

[defaults]
ports_per_project = 10
```

**`registry.toml`**

```toml
[[project]]
name = "my-frontend"
path = "/path/to/project"
ports = [3000, 3001]
range = [3000, 3010]

[[project]]
name = "shared"
ports = [5432, 6379]
```

## How It Works

- Uses the [`listeners`](https://docs.rs/listeners) crate for cross-platform port scanning — no shelling out to `lsof` or `netstat`
- Conflict detection compares PIDs, not socket counts — dual-stack bindings produce zero false positives
- Single static binary, zero runtime dependencies
- 105 tests covering edge cases (port 65535 overflow, inverted ranges, malformed config, cross-platform process management)
- CI runs on macOS, Linux, and Windows on every commit

## License

MIT
