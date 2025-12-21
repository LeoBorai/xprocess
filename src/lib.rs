use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use anyhow::{Result, bail};

/// Reference of a system process spawned by [`Process::spawn`]
///
/// # Example
///
/// ```ignore
/// use xprocess::Process;
///
/// fn main() {
///    let process = Process::spawn("sleep").expect("Failed to spawn process");
///    println!("Spawned process with PID: {}", process.pid());
///    process.kill().expect("Failed to kill process");
/// }
/// ```
///
pub struct Process {
    pid: u32,
}

impl Process {
    pub fn spawn<S: AsRef<OsStr>>(cmd: S) -> Result<Self> {
        let mut command = Self::build_command::<S, _, S>(cmd, []);
        Self::spawn_child_process(&mut command)
    }

    pub fn spawn_with_args<S, I, T>(cmd: S, args: I) -> Result<Self>
    where
        T: AsRef<OsStr>,
        I: IntoIterator<Item = T>,
        S: AsRef<OsStr>,
    {
        let mut command = Self::build_command(cmd, args);
        Self::spawn_child_process(&mut command)
    }

    fn build_command<S, I, T>(cmd: S, args: I) -> Command
    where
        T: AsRef<OsStr>,
        I: IntoIterator<Item = T>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(cmd);
        command.args(args);
        command
    }

    fn spawn_child_process(cmd: &mut Command) -> Result<Self> {
        let mut child = cmd
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

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    fn spawn_process() {
        let process = Process::spawn("sleep").expect("Failed to spawn process");
        assert!(process.pid() > 0);
        thread::sleep(Duration::from_millis(100));
        let result = process.kill();
        assert!(result.is_ok(), "Failed to kill the process");
    }

    #[test]
    fn spawn_process_with_args() {
        let process = Process::spawn_with_args("sleep", ["1"]).expect("Failed to spawn process");
        assert!(process.pid() > 0);
        thread::sleep(Duration::from_millis(100));
        let result = process.kill();
        assert!(result.is_ok(), "Failed to kill the process");
    }
}
