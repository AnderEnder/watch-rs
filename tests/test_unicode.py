import os
import unittest
os.environ.pop("NO_COLOR", None)
from terminal_support import Terminal


class UnicodeTests(unittest.TestCase):
    def test_clip_at_display_columns_without_splitting_unicode(self):
        for char, count in [('界', 50), ('é', 101), ('e\u0301', 101), ('👩\u200d💻', 50)]:
            with self.subTest(char=char):
                with Terminal(['-t', f"printf '{char}%.0s' $(seq 1 150)"], width=101) as terminal:
                    expected = ('\r' + char * count).encode() + b'\x1b[1;1H'
                    terminal.wait_for(lambda: expected in terminal.output)
                    terminal.quit()

    def test_highlight_sequences_do_not_consume_columns_or_lose_reset(self):
        with Terminal(['-t', '-d', "printf 'x%.0s' $(seq 1 150)"], width=101) as terminal:
            terminal.wait_for(lambda: b'x' * 101 + b'\x1b[0m' in terminal.output)
            terminal.quit()

    def test_hyperlinks_keep_their_visible_text(self):
        command = r"printf '\033]8;;https://example.com\033\\label\033]8;;\033\\ tail'"
        with Terminal(['-t', command]) as terminal:
            terminal.wait_for(lambda: b'\rlabel tail\x1b[1;1H' in terminal.output)
            terminal.quit()
