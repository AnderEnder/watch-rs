use std::process::Output;

/// Display both captured streams and make silent command failures visible.
/// The streams are captured separately, so their original interleaving is unknown.
pub fn format_output(output: &Output) -> String {
    let mut content = String::from_utf8_lossy(&output.stdout).into_owned();
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
