use crate::watch_state::Frame;
use std::io::{self, Write};
use termion::{color, cursor};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Render a frame for a supplied terminal size, without reading terminal state.
pub fn render<W: Write>(
    output: &mut W,
    (width, height): (u16, u16),
    status_begin: &str,
    command: &str,
    now: &str,
    frame: &Frame,
    no_title: bool,
) -> io::Result<()> {
    if width == 0 || height == 0 {
        return output.flush();
    }

    if !no_title {
        let title = format!("{status_begin}{command}");
        let padding = (width as usize).saturating_sub(title.len() + now.len() + 1);
        let status = format!("{title}{}{now}", " ".repeat(padding));
        let status: String = status.chars().take(width as usize).collect();
        write!(output, "{status}\r")?;
        if height > 1 {
            writeln!(output)?;
        }
    }

    let available_height = if no_title {
        height
    } else {
        height.saturating_sub(2)
    };
    let highlight_start = color::Fg(color::Red).to_string();
    for (index, line) in frame
        .visible_lines(available_height as usize, !highlight_start.is_empty())
        .enumerate()
    {
        let prefix = if no_title && index == 0 { "\r" } else { "\r\n" };
        let clipped = clip_line(line.text, width as usize);
        if line.highlighted && !highlight_start.is_empty() {
            write!(output, "{prefix}{highlight_start}{clipped}\x1b[0m")?;
        } else {
            write!(output, "{prefix}{clipped}")?;
        }
    }

    write!(output, "{}", cursor::Goto(1, 1))?;
    output.flush()
}

/// Clip at terminal columns, preserving graphemes and complete SGR sequences.
pub fn clip_line(line: &str, width: usize) -> String {
    let mut result = String::new();
    let mut remaining = line;
    let mut columns = 0;
    let mut styled = false;
    'segments: while !remaining.is_empty() {
        if remaining.starts_with("\x1b[") {
            // CSI sequences end at an ASCII byte in 0x40..=0x7e.
            let Some(end) = remaining[2..].find(|c: char| ('\x40'..='\x7e').contains(&c)) else {
                break;
            };
            let end = end + 3;
            let sequence = &remaining[..end];
            if sequence.ends_with('m') {
                result.push_str(sequence);
                styled = true;
            }
            remaining = &remaining[end..];
            continue;
        }
        let end = remaining.find('\x1b').unwrap_or(remaining.len());
        if end == 0 {
            // OSC hyperlinks and other control strings have no display width.
            if remaining.starts_with("\x1b]") || remaining.starts_with("\x1bP") {
                let body = &remaining[2..];
                let terminator = body
                    .find('\x07')
                    .map(|i| (i, 1))
                    .into_iter()
                    .chain(body.find("\x1b\\").map(|i| (i, 2)))
                    .min_by_key(|&(i, _)| i);
                let Some((end, length)) = terminator else {
                    break;
                };
                remaining = &body[end + length..];
            } else {
                // Consume an ESC sequence's intermediate and final bytes.
                let Some(end) = remaining[1..].find(|c: char| ('\x30'..='\x7e').contains(&c))
                else {
                    break;
                };
                remaining = &remaining[end + 2..];
            }
            continue;
        }
        for grapheme in remaining[..end].graphemes(true) {
            let size = if grapheme == "\t" {
                8 - columns % 8
            } else {
                UnicodeWidthStr::width(grapheme)
            };
            if size > width.saturating_sub(columns) {
                break 'segments;
            }
            if grapheme == "\t" {
                result.push_str(&" ".repeat(size));
            } else if !grapheme.chars().any(char::is_control) {
                result.push_str(grapheme);
            }
            columns += size;
        }
        remaining = &remaining[end..];
    }
    if styled {
        result.push_str("\x1b[0m");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::watch_state::WatchState;
    use termion::{color, cursor};

    #[test]
    fn renderer_clips_highlighted_lines_and_resets_color() {
        let mut state = WatchState::new(true, false);
        state.update("wide".to_owned());
        let mut output = Vec::new();

        render(&mut output, (2, 1), "", "", "", state.frame(), true).unwrap();

        let red = color::Fg(color::Red).to_string();
        let expected = if red.is_empty() {
            format!("\rwi{}", cursor::Goto(1, 1))
        } else {
            format!("\r{red}wi\x1b[0m{}", cursor::Goto(1, 1))
        };
        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn renderer_drops_only_the_last_unstyled_blank_difference_line() {
        let mut state = WatchState::new(true, false);
        state.update("same\n\n".to_owned());
        state.update("same".to_owned());
        let mut output = Vec::new();
        render(&mut output, (10, 3), "", "", "", state.frame(), true).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "\rsame\x1b[1;1H");

        state.update("same\n\n\n".to_owned());
        state.update("same".to_owned());
        let mut output = Vec::new();
        render(&mut output, (10, 3), "", "", "", state.frame(), true).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "\rsame\r\n\x1b[1;1H");
    }

    #[test]
    fn cumulative_display_follows_the_newest_lines() {
        let mut state = WatchState::new(false, true);
        state.update("one\ntwo\nthree".to_owned());
        let mut output = Vec::new();

        render(&mut output, (10, 2), "", "", "", state.frame(), true).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\rtwo\r\nthree\x1b[1;1H"
        );
    }

    #[test]
    fn cumulative_trim_does_not_expose_partial_ansi_sequence() {
        for escape in ["\x1b[31m", "\x1b]8;;https://example.com\x1b\\"] {
            let mut state = WatchState::new(false, true);
            state.update(format!(
                "{escape}{}",
                "x".repeat(4 * 1024 * 1024 + 2 - escape.len())
            ));
            let mut output = Vec::new();

            render(&mut output, (10, 1), "", "", "", state.frame(), true).unwrap();

            assert_eq!(String::from_utf8(output).unwrap(), "\rxxxxxxxxxx\x1b[1;1H");
        }
    }
}
