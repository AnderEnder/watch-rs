use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// Own the shell's process group until its output has been collected or cancelled.
pub struct RunningCommand {
    child: Child,
    stdout: Receiver<io::Result<Vec<u8>>>,
    stderr: Receiver<io::Result<Vec<u8>>>,
    stdout_bytes: Option<Vec<u8>>,
    stderr_bytes: Option<Vec<u8>>,
    finished: bool,
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
}

fn collect(
    receiver: &Receiver<io::Result<Vec<u8>>>,
    bytes: &mut Option<Vec<u8>>,
) -> io::Result<()> {
    if bytes.is_none() {
        match receiver.try_recv() {
            Ok(result) => *bytes = Some(result?),
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
            stdout_bytes: None,
            stderr_bytes: None,
            finished: false,
        })
    }

    pub fn poll(&mut self) -> io::Result<Option<Output>> {
        collect(&self.stdout, &mut self.stdout_bytes)?;
        collect(&self.stderr, &mut self.stderr_bytes)?;
        // Keep the shell unreaped while descendants still hold the pipes. This
        // also keeps its process-group ID reserved until cancellation is safe.
        if self.stdout_bytes.is_none() || self.stderr_bytes.is_none() {
            return Ok(None);
        }
        let Some(status) = self.child.try_wait()? else {
            return Ok(None);
        };
        self.finished = true;
        Ok(Some(Output {
            status,
            stdout: self.stdout_bytes.take().unwrap(),
            stderr: self.stderr_bytes.take().unwrap(),
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
