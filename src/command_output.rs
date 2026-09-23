use crate::running_command::{CapturedOutput, MAX_CAPTURE_BYTES};

/// Display both captured streams and make silent command failures visible.
/// The streams are captured separately, so their original interleaving is unknown.
pub fn format_output(output: &CapturedOutput) -> String {
    let mut content = String::new();
    if output.stdout_truncated {
        content.push_str(&format!(
            "[stdout truncated after {MAX_CAPTURE_BYTES} bytes]\n"
        ));
    }
    if output.stderr_truncated {
        content.push_str(&format!(
            "[stderr truncated after {MAX_CAPTURE_BYTES} bytes]\n"
        ));
    }
    content.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("[command failed: {}]\n", output.status));
    }
    content
}
