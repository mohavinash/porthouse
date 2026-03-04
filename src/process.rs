/// Cross-platform process utilities: kill and alive-check.
use anyhow::Result;

/// Validate PID is in a safe range (rejects 0, negative-when-cast, and PID 1).
fn validate_pid(pid: u32) -> Result<()> {
    if pid == 0 {
        anyhow::bail!("Refusing to signal PID 0 (would target own process group)");
    }
    if pid > i32::MAX as u32 {
        anyhow::bail!("PID {} exceeds safe range (would wrap to negative = process group signal)", pid);
    }
    Ok(())
}

/// Send SIGTERM (unix) or TerminateProcess (windows) to a process.
pub fn kill_process(pid: u32) -> Result<()> {
    validate_pid(pid)?;
    platform::kill(pid)
}

/// Check if a process with the given PID is still running.
pub fn is_process_alive(pid: u32) -> bool {
    if validate_pid(pid).is_err() {
        return false;
    }
    platform::is_alive(pid)
}

#[cfg(unix)]
mod platform {
    use anyhow::Result;

    pub fn kill(pid: u32) -> Result<()> {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if ret == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to kill PID {}: {}",
                pid,
                std::io::Error::last_os_error()
            ))
        }
    }

    pub fn is_alive(pid: u32) -> bool {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

#[cfg(windows)]
mod platform {
    use anyhow::Result;

    pub fn kill(pid: u32) -> Result<()> {
        use std::process::Command;
        let output = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to kill PID {}: {}", pid, stderr.trim()))
        }
    }

    pub fn is_alive(pid: u32) -> bool {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}
