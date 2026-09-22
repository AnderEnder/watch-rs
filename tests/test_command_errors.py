import unittest
from terminal_support import Terminal


class CommandErrorTests(unittest.TestCase):
    def test_stderr_is_visible(self):
        with Terminal(['echo error-message >&2']) as terminal:
            terminal.wait_for(lambda: b'\r\nerror-message' in terminal.output)
            terminal.quit()

    def test_both_streams_and_failure_are_visible(self):
        with Terminal(["printf out; printf err >&2; exit 7"]) as terminal:
            terminal.wait_for(lambda: b'\r\nout\r\nerr\r\n[command failed: exit status: 7]' in terminal.output)
            terminal.quit()

    def test_silent_failure_is_visible(self):
        with Terminal(['exit 7']) as terminal:
            terminal.wait_for(lambda: b'\r\n[command failed: exit status: 7]' in terminal.output)
            terminal.quit()

    def test_signal_failure_is_visible(self):
        with Terminal(['kill -TERM $$']) as terminal:
            terminal.wait_for(lambda: b'\r\n[command failed:' in terminal.output)
            terminal.quit()

    def test_successful_stdout_does_not_report_failure(self):
        with Terminal(['echo good']) as terminal:
            terminal.wait_for(lambda: b'\r\ngood' in terminal.output)
            self.assertNotIn(b'[command failed:', terminal.output)
            terminal.quit()
