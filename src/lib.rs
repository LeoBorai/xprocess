use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use anyhow::{Result, bail};

/// Reference of a system process spawned by [`Process::spawn`]
pub struct Process {
    pid: u32,
}

impl Process {
    pub fn spawn<S: AsRef<OsStr>>(cmd: S) -> Result<Self> {
        let mut child = Command::new(cmd);

        let mut child = child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        unsafe {
            child = child.pre_exec(|| {
                // Create a new session to detach the process
                libc::setsid();
                Ok(())
            });
        }

        let child_process = child.spawn()?;
        let pid = child_process.id();

        Ok(Self { pid })
    }

    /// Retrieves PID for the spawned process
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Kills the process referenced by this instance of [`Process`]
    pub fn kill(&self) -> Result<()> {
        match Command::new("kill").arg(self.pid().to_string()).status() {
            Ok(status) => {
                if status.success() {
                    return Ok(());
                }

                bail!("Failed to kill process with PID: {}", self.pid());
            }
            Err(e) => {
                bail!("Error executing kill command: {}", e);
            }
        }
    }
}
