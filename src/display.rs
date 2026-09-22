use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
