"""Exercise the real binary in a controlling terminal; Python 3 stdlib only."""
import fcntl
import os
import pty
import select
import signal
import struct
import termios
import time


class Terminal:
    def __init__(self, args, width=160, height=24):
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            fcntl.ioctl(1, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
            os.execv(os.environ["WATCH_BINARY"], ["watch", *args])
        self.output = b""
        self.status = None

    def __enter__(self):
        return self

    def pump(self, timeout=0.02):
        if select.select([self.fd], [], [], timeout)[0]:
            try:
                self.output += os.read(self.fd, 65536)
            except OSError:
                pass
        if self.status is None:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.status = os.waitstatus_to_exitcode(status)

    def wait_for(self, predicate, timeout=3):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.pump()
            if predicate():
                return
            if self.status is not None:
                break
        raise AssertionError(f"terminal condition not met; status={self.status}, output={self.output[-2000:]!r}")

    def send(self, data):
        os.write(self.fd, data)

    def resize(self, width, height):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))

    def quit(self, key=b"q"):
        self.send(key)
        self.wait_for(lambda: self.status is not None)
        assert self.status == 0, self.output

    def __exit__(self, *_):
        # Terminate only the session created for this test, including descendants.
        try:
            os.killpg(self.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        if self.status is None:
            os.waitpid(self.pid, 0)
        os.close(self.fd)
