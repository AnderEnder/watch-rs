use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// Own the shell's process group until its output has been collected or cancelled.
pub struct RunningCommand {
    child: Child,
    stdout: Receiver<io::Result<CapturedPipe>>,
    stderr: Receiver<io::Result<CapturedPipe>>,
    stdout_output: Option<CapturedPipe>,
    stderr_output: Option<CapturedPipe>,
    finished: bool,
}

pub(crate) const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

struct CapturedPipe {
    bytes: Vec<u8>,
    truncated: bool,
}

pub struct CapturedOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> Receiver<io::Result<CapturedPipe>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut truncated = false;
        let result = (|| {
            let mut chunk = [0; 8192];
            loop {
                let count = match pipe.read(&mut chunk) {
                    Ok(count) => count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                };
                if count == 0 {
                    break;
                }
                let keep = count.min(MAX_CAPTURE_BYTES - bytes.len());
                bytes.extend_from_slice(&chunk[..keep]);
                truncated |= keep < count;
            }
            Ok(CapturedPipe { bytes, truncated })
        })();
        let _ = sender.send(result);
    });
    receiver
}

fn collect(
    receiver: &Receiver<io::Result<CapturedPipe>>,
    output: &mut Option<CapturedPipe>,
) -> io::Result<()> {
    if output.is_none() {
        match receiver.try_recv() {
            Ok(result) => *output = Some(result?),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                return Err(io::Error::other("output reader stopped"));
            }
        }
    }
    Ok(())
}

impl RunningCommand {
    pub fn spawn(command: &str) -> io::Result<Self> {
        let mut child = Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?;
        let stdout = read_pipe(child.stdout.take().expect("stdout is piped"));
        let stderr = read_pipe(child.stderr.take().expect("stderr is piped"));
        Ok(Self {
            child,
            stdout,
            stderr,
            stdout_output: None,
            stderr_output: None,
            finished: false,
        })
    }

    pub fn poll(&mut self) -> io::Result<Option<CapturedOutput>> {
        collect(&self.stdout, &mut self.stdout_output)?;
        collect(&self.stderr, &mut self.stderr_output)?;
        // Keep the shell unreaped while descendants still hold the pipes. This
        // also keeps its process-group ID reserved until cancellation is safe.
        if self.stdout_output.is_none() || self.stderr_output.is_none() {
            return Ok(None);
        }
        let Some(status) = self.child.try_wait()? else {
            return Ok(None);
        };
        self.finished = true;
        let stdout = self.stdout_output.take().unwrap();
        let stderr = self.stderr_output.take().unwrap();
        Ok(Some(CapturedOutput {
            status,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            stdout_truncated: stdout.truncated,
            stderr_truncated: stderr.truncated,
        }))
    }
}

impl Drop for RunningCommand {
    fn drop(&mut self) {
        if !self.finished {
            // SAFETY: the child leads a separate process group created above;
            // its unreaped PID remains reserved, and kill accepts no pointers.
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn large_streams_are_bounded_and_report_truncation() {
        let mut running = RunningCommand::spawn(
            "python3 -c 'import sys; sys.stdout.buffer.write(b\"x\" * 1200000); sys.stderr.buffer.write(b\"y\" * 1200000)'",
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let output = loop {
            if let Some(output) = running.poll().unwrap() {
                break output;
            }
            assert!(
                Instant::now() < deadline,
                "command did not finish after its pipes filled"
            );
            thread::sleep(Duration::from_millis(1));
        };

        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 1_048_576);
        assert_eq!(output.stderr.len(), 1_048_576);
        let display = crate::command_output::format_output(&output);
        assert!(display.starts_with("[stdout truncated after 1048576 bytes]\n"));
        assert!(display.contains("[stderr truncated after 1048576 bytes]\n"));
    }
}
