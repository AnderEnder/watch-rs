/// The current text and its display styling, independent of terminal I/O.
#[derive(Default)]
pub struct Frame {
    text: String,
    highlighted: Vec<bool>,
    extra_blank_lines: usize,
    joined_diff: bool,
}

pub struct FrameLine<'a> {
    pub text: &'a str,
    pub highlighted: bool,
}

impl Frame {
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
    previous_content: String,
    cumulative_content: String,
    frame: Frame,
}

impl WatchState {
    pub fn new(difference: bool, cumulative: bool) -> Self {
        Self {
            difference,
            cumulative,
            previous_content: String::new(),
            cumulative_content: String::new(),
            frame: Frame::default(),
        }
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn update(&mut self, raw_content: String) {
        let old_content = if self.cumulative {
            &self.cumulative_content
        } else {
            &self.previous_content
        };
        let content = if self.cumulative && !old_content.is_empty() {
            format!("{old_content}\n{raw_content}")
        } else {
            raw_content
        };
        let joined_diff = self.difference && old_content != &content;
        let (highlighted, extra_blank_lines) = if joined_diff {
            changed_lines(old_content, &content)
        } else {
            (Vec::new(), 0)
        };
        if self.cumulative {
            self.cumulative_content = content.clone();
        } else if self.difference {
            self.previous_content = content.clone();
        }
        self.frame = Frame {
            text: content,
            highlighted,
            extra_blank_lines,
            joined_diff,
        };
    }
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
}
