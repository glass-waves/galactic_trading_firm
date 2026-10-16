//! human notification from the trader process: `scripts/notify.sh <level> "<message>"`
//! (ntfy + `logs/notifications.log`). fire-and-forget on a detached thread — never
//! blocks the tick loop and never panics; a missing script is logged and ignored.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use tracing::{error, warn};

/// where `notify.sh` lives: `NOTIFY_SCRIPT` if set, else `<cwd>/scripts/notify.sh`
/// (the systemd unit sets `WorkingDirectory` to the repo root), else relative to the
/// binary (`target/<profile>/paper_trader` → `../../scripts/notify.sh`).
pub fn notify_script_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("NOTIFY_SCRIPT") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("scripts").join("notify.sh");
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let p = exe
            .parent()
            .and_then(|d| d.parent())
            .and_then(|d| d.parent())
            .map(|root| root.join("scripts").join("notify.sh"));
        if let Some(p) = p.filter(|p| p.is_file()) {
            return Some(p);
        }
    }
    None
}

/// send `message` at `level` ("info" | "warning" | "critical"). the script runs on a
/// detached thread (reaped there, so no zombie); nothing here can fail the caller.
pub fn notify(level: &'static str, message: String) {
    let Some(script) = notify_script_path() else {
        warn!(level, message = %message, "notify.sh not found — notification only logged");
        return;
    };
    let spawn = std::thread::Builder::new()
        .name("notify".into())
        .spawn(move || {
            let status = Command::new(&script)
                .arg(level)
                .arg(&message)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if let Err(e) = status {
                error!(script = %script.display(), error = %e, "notify.sh failed to run");
            }
        });
    if let Err(e) = spawn {
        error!(error = %e, "could not spawn the notify thread");
    }
}

/// WARNING to the human: something the trader handled itself but wants looked at.
pub fn notify_warning(message: String) {
    warn!(severity = "WARNING", message = %message, "WARNING");
    notify("warning", message);
}

/// CRITICAL to the human: the broker and the engine disagree about what is held.
pub fn notify_critical(message: String) {
    error!(severity = "CRITICAL", message = %message, "CRITICAL");
    notify("critical", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// one test (not two) because both touch the same process-wide env var.
    #[test]
    fn notify_never_panics_and_prefers_the_env_override() {
        std::env::set_var("NOTIFY_SCRIPT", "/nonexistent/notify.sh");
        notify("info", "test".to_string());
        notify_critical("test".to_string());

        let dir = std::env::temp_dir().join(format!("gtf-notify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("notify.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        std::env::set_var("NOTIFY_SCRIPT", &script);
        assert_eq!(notify_script_path(), Some(script.clone()));
        notify("info", "ok".to_string());
        std::env::remove_var("NOTIFY_SCRIPT");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
