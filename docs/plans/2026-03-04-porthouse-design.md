# Porthouse Design

*A lighthouse for your ports — monitors, routes, and resolves conflicts across projects.*

## Problem

When running multiple full-stack projects simultaneously (frontend + backend + database each), port conflicts are constant. No existing tool combines real-time monitoring, conflict detection, project-aware port registry, and alerting in a lightweight package.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Porthouse System                   │
├──────────────┬──────────────┬───────────────────────┤
│   Daemon     │    TUI       │    CLI                │
│  (always on) │ (on-demand)  │  (scriptable)         │
├──────────────┴──────────────┴───────────────────────┤
│                    Core Library                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ Port     │ │ Registry │ │ Conflict │            │
│  │ Scanner  │ │ Manager  │ │ Resolver │            │
│  └──────────┘ └──────────┘ └──────────┘            │
├─────────────────────────────────────────────────────┤
│                  Alert System                        │
│  macOS Notif │ Terminal │ Log File │ Webhook        │
└─────────────────────────────────────────────────────┘
```

### Components

| Component | What it does | Footprint |
|-----------|-------------|-----------|
| `porthouse` | TUI dashboard (Ratatui) | < 5 MB RAM, on-demand |
| `porthouse daemon` | Background port scanner + alerter | < 3 MB RAM, < 0.1% CPU |
| `porthouse <cmd>` | Scriptable CLI commands | One-shot, exits immediately |

## Core Features

1. **Real-time port monitoring** — scans every 3s via `libproc` (macOS) / `procfs` (Linux), shows all listening ports with PID/process/project
2. **Conflict detection + resolution** — detects same-port collisions, suggests free alternatives, one-key kill/move
3. **Project registry** — `~/.porthouse/registry.toml` maps projects to port ranges
4. **Alert system** — macOS notifications, terminal banners, log file, optional webhook
5. **Claude Code hooks** — auto-checks for conflicts before dev server starts, suggests free ports

## Tech Stack

- **Language:** Rust
- **TUI:** Ratatui + Crossterm
- **Port scanning:** `libproc` (macOS) / `procfs` (Linux) — direct syscalls, no shelling out
- **Config:** TOML (`toml` crate)
- **Notifications:** `notify-rust` (macOS/Linux native)
- **No async runtime** — `std::thread::sleep` for the daemon scan loop

## TUI Layout

```
┌─ Porthouse ──────────────────────────────────────────────────┐
│ ┌─ Active Ports ───────────────────────────────────────────┐ │
│ │ PORT   PID    PROCESS          PROJECT     STATUS        │ │
│ │ 3000   1234   node (next)      WA Dashboard  ● OK       │ │
│ │ 3001   1235   node (next)      WA Dashboard  ● OK       │ │
│ │ 5432   892    postgres         shared        ● OK        │ │
│ │ 8000   2341   python (flask)   theatrelabs   ● OK        │ │
│ │ 8000   2456   python (uvicorn) IndoorMap      ⚠ CONFLICT │ │
│ │ 6379   445    redis-server     shared        ● OK        │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌─ Conflicts (1) ─────────────────────────────────────────┐  │
│ │ Port 8000: theatrelabs (PID 2341) vs IndoorMap (PID 2456)│ │
│ │ Suggestion: Move IndoorMap to port 8001 (free)           │ │
│ │ [K]ill PID  [M]ove to suggested  [I]gnore               │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌─ Registry ──────────────────────────────────────────────┐  │
│ │ WA Dashboard:  3000-3010    theatrelabs:  8000-8010     │ │
│ │ IndoorMap:     8100-8110    WallBreaker:  4000-4010     │ │
│ └──────────────────────────────────────────────────────────┘ │
│ [q]uit  [r]efresh  [a]dd project  [s]uggest port  [?]help   │
└──────────────────────────────────────────────────────────────┘
```

## CLI Commands

```
porthouse                          # Open TUI
porthouse daemon start|stop|status # Manage daemon
porthouse status                   # One-shot port listing
porthouse check                    # Conflict check (exit 1 if any)
porthouse suggest [N]              # Suggest N free ports
porthouse register <name> [range]  # Register project
porthouse kill <port>              # Kill process on port
porthouse free <port>              # Check if port is free
```

## Configuration

All files in `~/.porthouse/`:

### `config.toml`
```toml
[daemon]
scan_interval_secs = 3
port_range = [1024, 65535]

[alerts]
macos_notifications = true
terminal_bell = true
log_file = "~/.porthouse/alerts.log"
webhook_url = ""

[defaults]
ports_per_project = 10
```

### `registry.toml`
```toml
[[project]]
name = "WA Dashboard"
path = "/Users/avinash/AICodeLab/WA Dashboard"
ports = [3000, 3001, 3002]
range = [3000, 3010]

[[project]]
name = "theatrelabs"
path = "/Users/avinash/AICodeLab/theatrelabs"
ports = [8000, 8001]
range = [8000, 8010]

[[project]]
name = "shared"
ports = [5432, 6379]
```

## Claude Code Integration

Hook in `~/.claude/settings.json`:
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "command": "portguard check --quiet || echo 'WARNING: Port conflict detected. Run porthouse status for details.'"
      }
    ]
  }
}
```

## Performance Targets

- Memory: < 5 MB RSS
- CPU: < 0.1% average (sleep between 3s scans)
- Binary: < 3 MB (static Rust binary)
- Disk: config + registry < 1 KB, log rotated at 1 MB
- No embedded database, no async runtime, no network server
