/// Limit the history that each cumulative refresh copies and compares.
const MAX_CUMULATIVE_BYTES: usize = 4 * 1024 * 1024;
const MAX_CUMULATIVE_LINES: usize = u16::MAX as usize;

/// The current text and its display styling, independent of terminal I/O.
#[derive(Default)]
pub struct Frame {
    text: String,
    highlighted: Vec<bool>,
    extra_blank_lines: usize,
    joined_diff: bool,
    follow_tail: bool,
}

pub struct FrameLine<'a> {
    pub text: &'a str,
    pub highlighted: bool,
}

impl Frame {
    pub fn visible_lines(
        &self,
        height: usize,
        show_highlights: bool,
    ) -> impl Iterator<Item = FrameLine<'_>> {
        let skip = if self.follow_tail {
            self.lines(show_highlights).count().saturating_sub(height)
        } else {
            0
        };
        self.lines(show_highlights).skip(skip).take(height)
    }

    pub fn lines(&self, show_highlights: bool) -> impl Iterator<Item = FrameLine<'_>> {
        let last_difference_line = if self.joined_diff {
            self.highlighted.len().checked_sub(1)
        } else {
            None
        };
        self.text
            .lines()
            .chain(std::iter::repeat_n("", self.extra_blank_lines))
            .enumerate()
            .filter_map(move |(index, text)| {
                let highlighted = self.highlighted.get(index).copied().unwrap_or(false);
                // The old string-based diff omitted its final empty line when it
                // contained no color codes. Preserve that display behavior.
                if Some(index) == last_difference_line
                    && text.is_empty()
                    && (!highlighted || !show_highlights)
                {
                    return None;
                }
                Some(FrameLine { text, highlighted })
            })
    }
}

pub struct WatchState {
    difference: bool,
    cumulative: bool,
    frame: Frame,
}

impl WatchState {
    pub fn new(difference: bool, cumulative: bool) -> Self {
        Self {
            difference,
            cumulative,
            frame: Frame::default(),
        }
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn update(&mut self, raw_content: String) {
        let old_content = &self.frame.text;
        let mut content = if self.cumulative && !old_content.is_empty() {
            format!("{old_content}\n{raw_content}")
        } else {
            raw_content
        };
        let joined_diff = self.difference && old_content != &content;
        let discarded = if self.cumulative {
            trim_cumulative(&mut content)
        } else {
            0
        };
        let old_visible = old_content.get(discarded..).unwrap_or("");
        let (highlighted, extra_blank_lines) = if joined_diff {
            changed_lines(old_visible, &content)
        } else {
            (Vec::new(), 0)
        };
        self.frame = Frame {
            text: content,
            highlighted,
            extra_blank_lines,
            joined_diff,
            follow_tail: self.cumulative,
        };
    }
}

fn trim_cumulative(content: &mut String) -> usize {
    let mut start = content.len().saturating_sub(MAX_CUMULATIVE_BYTES);
    while !content.is_char_boundary(start) {
        start += 1;
    }
    start = after_partial_escape(content.as_bytes(), start);
    let tail = &content[start..];
    let excess_lines = tail.lines().count().saturating_sub(MAX_CUMULATIVE_LINES);
    if excess_lines > 0 {
        let (newline, _) = tail
            .match_indices('\n')
            .nth(excess_lines - 1)
            .expect("excess lines have a newline to discard");
        start += newline + 1;
    }
    content.drain(..start);
    start
}

fn after_partial_escape(bytes: &[u8], start: usize) -> usize {
    // Rendering parses each line separately. Only an escape on the cutoff's
    // line can leave a visible fragment when its prefix is discarded.
    let line_start = bytes[..start]
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |position| position + 1);
    let line_end = bytes[start..]
        .iter()
        .position(|&byte| byte == b'\n')
        .map_or(bytes.len(), |position| start + position);
    let line = &bytes[line_start..line_end];
    let cutoff = start - line_start;
    let mut cursor = 0;
    while let Some(offset) = line[cursor..cutoff]
        .iter()
        .position(|&byte| byte == b'\x1b')
    {
        let escape = cursor + offset;
        let end = match line.get(escape + 1) {
            Some(b'[') => line[escape + 2..]
                .iter()
                .position(|&byte| (0x40..=0x7e).contains(&byte))
                .map(|offset| escape + offset + 3),
            Some(b']' | b'P') => {
                let mut position = escape + 2;
                let mut end = None;
                while position < line.len() {
                    if line[position] == b'\x07' {
                        end = Some(position + 1);
                        break;
                    }
                    if line[position] == b'\x1b' && line.get(position + 1) == Some(&b'\\') {
                        end = Some(position + 2);
                        break;
                    }
                    position += 1;
                }
                end
            }
            _ => line[escape + 1..]
                .iter()
                .position(|&byte| (0x30..=0x7e).contains(&byte))
                .map(|offset| escape + offset + 2),
        }
        .unwrap_or(line.len());
        if end > cutoff {
            return if end == line.len() && line_end < bytes.len() {
                line_end + 1
            } else {
                line_start + end
            };
        }
        cursor = end;
    }
    start
}

fn changed_lines(old_content: &str, new_content: &str) -> (Vec<bool>, usize) {
    let old_lines: Vec<_> = old_content.lines().collect();
    let new_lines: Vec<_> = new_content.lines().collect();
    let count = old_lines.len().max(new_lines.len());
    let highlighted = (0..count)
        .map(|index| {
            old_lines.get(index).copied().unwrap_or("")
                != new_lines.get(index).copied().unwrap_or("")
        })
        .collect();
    let extra_blank_lines = old_lines.len().saturating_sub(new_lines.len());
    (highlighted, extra_blank_lines)
}

#[cfg(test)]
mod tests {
    use super::WatchState;

    fn lines(state: &WatchState) -> Vec<(String, bool)> {
        state
            .frame()
            .lines(true)
            .map(|line| (line.text.to_owned(), line.highlighted))
            .collect()
    }

    #[test]
    fn differences_compare_completed_outputs_and_preserve_removed_line_slots() {
        let mut state = WatchState::new(true, false);
        state.update("same\nold".to_owned());
        assert_eq!(lines(&state), [("same".into(), true), ("old".into(), true)]);

        state.update("same\nnew".to_owned());
        assert_eq!(
            lines(&state),
            [("same".into(), false), ("new".into(), true)]
        );

        state.update("same".to_owned());
        assert_eq!(lines(&state), [("same".into(), false), ("".into(), true)]);
    }

    #[test]
    fn cumulative_mode_appends_completed_outputs() {
        let mut state = WatchState::new(false, true);
        state.update("first".to_owned());
        state.update("second".to_owned());
        assert_eq!(
            lines(&state),
            [("first".into(), false), ("second".into(), false)]
        );
    }

    #[test]
    fn cumulative_difference_marks_only_the_new_lines() {
        let mut state = WatchState::new(true, true);
        state.update("first".to_owned());
        state.update("second".to_owned());
        assert_eq!(
            lines(&state),
            [("first".into(), false), ("second".into(), true)]
        );
    }

    #[test]
    fn colorless_differences_drop_a_final_removed_line() {
        let mut state = WatchState::new(true, false);
        state.update("same\nold".to_owned());
        state.update("same".to_owned());

        assert_eq!(lines(&state), [("same".into(), false), ("".into(), true)]);
        assert_eq!(
            state
                .frame()
                .lines(false)
                .map(|line| line.text)
                .collect::<Vec<_>>(),
            ["same"]
        );
    }

    #[test]
    fn unchanged_output_keeps_its_trailing_blank_line() {
        let mut state = WatchState::new(true, false);
        state.update("same\n\n".to_owned());
        state.update("same\n\n".to_owned());

        assert_eq!(lines(&state), [("same".into(), false), ("".into(), false)]);
    }

    #[test]
    fn cumulative_history_keeps_recent_utf8_within_four_megabytes() {
        let mut state = WatchState::new(false, true);
        state.update("🙂".repeat(1_048_576));
        state.update("ok".to_owned());

        assert!(state.frame.text.len() <= 4 * 1024 * 1024);
        assert!(state.frame.text.starts_with('🙂'));
        assert!(state.frame.text.ends_with("\nok"));
    }

    #[test]
    fn cumulative_difference_keeps_surviving_lines_unhighlighted_after_trimming() {
        let mut state = WatchState::new(true, true);
        state.update(format!("{}\nstable", "x".repeat(4 * 1024 * 1024 - 10)));
        state.update("fresh".to_owned());

        assert!(state.frame.text.len() <= 4 * 1024 * 1024);
        let lines = lines(&state);
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].1);
        assert_eq!(lines[1], ("stable".into(), false));
        assert_eq!(lines[2], ("fresh".into(), true));
    }

    #[test]
    fn cumulative_history_bounds_small_lines_to_the_maximum_terminal_height() {
        let mut state = WatchState::new(false, true);
        state.update((0..70_000).map(|n| format!("{n}\n")).collect());

        let lines = lines(&state);
        assert_eq!(lines.len(), 65_535);
        assert_eq!(lines.first().unwrap().0, "4465");
        assert_eq!(lines.last().unwrap().0, "69999");
    }

    #[test]
    fn unterminated_escape_before_cutoff_does_not_erase_new_output() {
        let mut state = WatchState::new(false, true);
        let prefix = "\x1b]unfinished\n";
        state.update(format!(
            "{prefix}{}",
            "x".repeat(4 * 1024 * 1024 - prefix.len())
        ));
        state.update("fresh".to_owned());

        assert!(state.frame.text.ends_with("\nfresh"));
    }
}
