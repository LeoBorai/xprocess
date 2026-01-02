use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

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
    child: Option<Child>,
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        unsafe {
            child = child.pre_exec(|| {
                // Create a new session to detach the process
                libc::setsid();
                Ok(())
            });
        }

        let child_process = child.spawn()?;
        let pid = child_process.id();

        Ok(Self {
            pid,
            child: Some(child_process),
        })
    }

    /// Retrieves PID for the spawned process
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Reads and returns the stdout of the process
    ///
    /// This method reads all available output from stdout and returns it as a String.
    /// The method will block until the process closes its stdout stream.
    /// 
    /// **Important:** This method consumes the stdout handle. Subsequent calls will return 
    /// an empty String.
    ///
    /// **Note:** For processes that produce output and then continue running, consider 
    /// waiting for the process to finish or close stdout before calling this method, 
    /// otherwise it may block indefinitely.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut process = Process::spawn_with_args("echo", ["hello"]).expect("Failed to spawn");
    /// // Wait for the process to finish writing
    /// std::thread::sleep(std::time::Duration::from_millis(100));
    /// let output = process.stdout().expect("Failed to read stdout");
    /// assert_eq!(output.trim(), "hello");
    /// ```
    pub fn stdout(&mut self) -> Result<String> {
        if let Some(ref mut child) = self.child
            && let Some(ref mut stdout) = child.stdout
        {
            let mut output = String::new();
            stdout.read_to_string(&mut output)?;
            return Ok(output);
        }
        Ok(String::new())
    }

    /// Reads and returns the stderr of the process
    ///
    /// This method reads all available output from stderr and returns it as a String.
    /// The method will block until the process closes its stderr stream.
    /// 
    /// **Important:** This method consumes the stderr handle. Subsequent calls will return 
    /// an empty String.
    ///
    /// **Note:** For processes that produce output and then continue running, consider 
    /// waiting for the process to finish or close stderr before calling this method, 
    /// otherwise it may block indefinitely.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut process = Process::spawn_with_args("ls", ["/nonexistent"]).expect("Failed to spawn");
    /// // Wait for the process to finish writing
    /// std::thread::sleep(std::time::Duration::from_millis(100));
    /// let error = process.stderr().expect("Failed to read stderr");
    /// assert!(error.contains("No such file or directory"));
    /// ```
    pub fn stderr(&mut self) -> Result<String> {
        if let Some(ref mut child) = self.child
            && let Some(ref mut stderr) = child.stderr
        {
            let mut output = String::new();
            stderr.read_to_string(&mut output)?;
            return Ok(output);
        }
        Ok(String::new())
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

    #[test]
    fn spawn_process_with_args_different_types() {
        let process = Process::spawn_with_args("sleep", [String::from("1")])
            .expect("Failed to spawn process");
        assert!(process.pid() > 0);
        thread::sleep(Duration::from_millis(100));
        let result = process.kill();
        assert!(result.is_ok(), "Failed to kill the process");
    }

    #[test]
    fn capture_stdout() {
        let mut process =
            Process::spawn_with_args("echo", ["hello world"]).expect("Failed to spawn process");
        thread::sleep(Duration::from_millis(100));
        let stdout = process.stdout().expect("Failed to read stdout");
        assert_eq!(stdout.trim(), "hello world");
        process.kill().expect("Failed to kill process");
    }

    #[test]
    fn capture_stderr() {
        let mut process = Process::spawn_with_args("sh", ["-c", "echo error message >&2"])
            .expect("Failed to spawn process");
        thread::sleep(Duration::from_millis(100));
        let stderr = process.stderr().expect("Failed to read stderr");
        assert_eq!(stderr.trim(), "error message");
        process.kill().expect("Failed to kill process");
    }

    #[test]
    fn capture_both_stdout_and_stderr() {
        let mut process =
            Process::spawn_with_args("sh", ["-c", "echo stdout message; echo stderr message >&2"])
                .expect("Failed to spawn process");
        thread::sleep(Duration::from_millis(100));
        let stdout = process.stdout().expect("Failed to read stdout");
        let stderr = process.stderr().expect("Failed to read stderr");
        assert_eq!(stdout.trim(), "stdout message");
        assert_eq!(stderr.trim(), "stderr message");
        process.kill().expect("Failed to kill process");
    }

    #[test]
    fn capture_empty_stdout() {
        let mut process = Process::spawn_with_args("true", Vec::<String>::new())
            .expect("Failed to spawn process");
        thread::sleep(Duration::from_millis(100));
        let stdout = process.stdout().expect("Failed to read stdout");
        assert_eq!(stdout, "");
        process.kill().ok(); // Process might already be finished
    }
}
