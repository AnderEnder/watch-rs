import unittest
from terminal_support import Terminal


class TerminalSizeTests(unittest.TestCase):
    def test_small_dimensions_and_long_commands(self):
        for width, height, flags in [(40, 24, []), (40, 24, ['-t']), (160, 1, []), (0, 0, [])]:
            with self.subTest(width=width, height=height, flags=flags):
                with Terminal([*flags, 'echo ok # ' + 'x' * 200], width, height) as terminal:
                    terminal.wait_for(lambda: b'\x1b[1;1H' in terminal.output)
                    terminal.quit()

    def test_resize_to_narrow_terminal(self):
        with Terminal(['echo ok']) as terminal:
            terminal.wait_for(lambda: b'\r\nok' in terminal.output)
            before = len(terminal.output)
            terminal.resize(20, 1)
            terminal.wait_for(lambda: len(terminal.output) > before)
            terminal.quit()
