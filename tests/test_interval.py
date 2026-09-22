import os
import subprocess
import unittest
from terminal_support import Terminal


class IntervalTests(unittest.TestCase):
    def test_reject_invalid_intervals_before_terminal_setup(self):
        for interval in ['0', '-1', 'NaN', 'inf', '-inf', '1e100', '1e-100']:
            with self.subTest(interval=interval):
                result = subprocess.run([os.environ['WATCH_BINARY'], f'--interval={interval}', 'echo ok'], capture_output=True)
                self.assertEqual(result.returncode, 2, result.stderr)

    def test_short_interval_refreshes_and_remains_quittable(self):
        for interval in ['0.001', '0.000000001']:
            with self.subTest(interval=interval), Terminal(['-n', interval, 'echo tick']) as terminal:
                terminal.wait_for(lambda: terminal.output.count(b'\r\ntick') >= 3)
                terminal.quit()
