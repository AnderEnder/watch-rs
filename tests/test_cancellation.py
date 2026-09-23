import os
from pathlib import Path
import tempfile
import time
import unittest
from terminal_support import Terminal


class CancellationTests(unittest.TestCase):
    def test_quit_during_command_stops_descendants(self):
        for key in [b'q', b'\x03']:
            with self.subTest(key=key), tempfile.TemporaryDirectory(dir='/tmp') as directory:
                ready = Path(directory) / 'ready'
                leaked = Path(directory) / 'leaked'
                command = f'(sleep 0.7; touch {leaked}) & echo ready > {ready}; wait'
                with Terminal([command], width=400) as terminal:
                    terminal.wait_for(ready.exists)
                    terminal.send(key)
                    terminal.wait_for(lambda: terminal.status is not None, timeout=0.5)
                    self.assertEqual(terminal.status, 0)
                    time.sleep(0.8)
                    self.assertFalse(leaked.exists(), 'watched descendant survived quitting')

    def test_quit_when_shell_exits_but_descendant_holds_output(self):
        with Terminal(['sleep 30 &'], width=160) as terminal:
            terminal.wait_for(lambda: b'\x1b[1;1H' in terminal.output)
            terminal.quit()

    def test_quit_during_multiword_command_stops_descendants(self):
        with tempfile.TemporaryDirectory(dir='/tmp') as directory:
            ready = Path(directory) / 'ready'
            leaked = Path(directory) / 'leaked'
            command = f'(sleep 0.7; touch {leaked}) & echo ready > {ready}; wait'
            with Terminal(['sh', '-c', command], width=400) as terminal:
                terminal.wait_for(ready.exists)
                terminal.quit()
                time.sleep(0.8)
                self.assertFalse(leaked.exists(), 'multiword descendant survived quitting')

    def test_drains_both_output_pipes(self):
        command = 'i=0; while [ $i -lt 3000 ]; do echo stdout; echo stderr >&2; i=$((i+1)); done'
        with Terminal([command], width=400) as terminal:
            terminal.wait_for(lambda: b'\r\nstdout' in terminal.output)
            terminal.quit()

    def test_resize_while_command_runs(self):
        with Terminal(['sleep 30']) as terminal:
            terminal.wait_for(lambda: b'Every 2.00s:' in terminal.output)
            before = terminal.output.count(b'Every 2.00s:')
            terminal.resize(120, 24)
            terminal.wait_for(lambda: terminal.output.count(b'Every 2.00s:') > before)
            terminal.quit()
            self.assertIn(b'\x1b[?25h', terminal.output)
