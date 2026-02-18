use std::path::Path;
use std::process::Command;

pub fn get_commit_context(file_path: &Path) -> Option<String> {
    let parent = file_path.parent()?;

    let output = Command::new("git")
        .args(["log", "--format=%s", "-n", "50", "--"])
        .arg(file_path.file_name()?)
        .current_dir(parent)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    if messages.is_empty() {
        return None;
    }

    Some(format!("\n[git history]\n{}", messages.join("\n")))
}
