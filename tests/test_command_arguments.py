import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from terminal_support import Terminal


class CommandArgumentTests(unittest.TestCase):
    def test_multiple_words_preserve_spaces_inside_an_argument(self):
        with Terminal(['-t', '-n', '10', 'printf', '[%s]', 'a b']) as terminal:
            terminal.wait_for(lambda: b'\r[a b]\x1b[1;1H' in terminal.output)
            terminal.quit()

    def test_multiple_words_do_not_interpret_shell_syntax_in_arguments(self):
        with Terminal(['-t', '-n', '10', 'printf', '%s', '$(printf injected)']) as terminal:
            terminal.wait_for(lambda: b'\r$(printf injected)\x1b[1;1H' in terminal.output)
            terminal.quit()

    def test_single_command_string_still_runs_through_the_shell(self):
        with Terminal(['-t', '-n', '10', "printf 'hello'; printf ' world'"]) as terminal:
            terminal.wait_for(lambda: b'\rhello world\x1b[1;1H' in terminal.output)
            terminal.quit()

    def test_missing_program_reports_failure_without_exiting_watch(self):
        with Terminal(['-t', '-n', '10', 'watch-rs-program-that-does-not-exist', 'arg']) as terminal:
            terminal.wait_for(lambda: b'[command failed:' in terminal.output)
            self.assertIsNone(terminal.status)
            terminal.quit()

    def test_program_name_beginning_with_dash_is_not_an_exec_option(self):
        with tempfile.TemporaryDirectory() as directory:
            program = Path(directory) / '-c'
            program.write_text('#!/bin/sh\nprintf chosen\n')
            program.chmod(0o755)
            with patch.dict(os.environ, PATH=directory + os.pathsep + os.environ['PATH']):
                with Terminal(['-t', '-n', '10', '--', '-c', 'ignored']) as terminal:
                    terminal.wait_for(lambda: b'\rchosen\x1b[1;1H' in terminal.output)
                    terminal.quit()

    def test_shell_builtin_does_not_interpret_an_argument(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / 'executed'
            with Terminal(['-t', '-n', '10', 'eval', f'touch {marker}']) as terminal:
                terminal.wait_for(lambda: b'[command failed:' in terminal.output)
                self.assertFalse(marker.exists())
                terminal.quit()
