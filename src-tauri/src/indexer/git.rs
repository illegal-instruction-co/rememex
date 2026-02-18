use std::path::Path;

pub fn get_commit_context(file_path: &Path) -> Option<String> {
    let repo = git2::Repository::discover(file_path.parent()?).ok()?;
    let workdir = repo.workdir()?;
    let relative_path = file_path.strip_prefix(workdir).ok()?;

    let mut revwalk = repo.revwalk().ok()?;
    revwalk.push_head().ok()?;
    revwalk.set_sorting(git2::Sort::TIME).ok()?;

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.pathspec(relative_path);

    let mut messages = Vec::new();

    for oid in revwalk.flatten() {
        if messages.len() >= 50 {
            break;
        }

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let diff =
            match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts)) {
                Ok(d) => d,
                Err(_) => continue,
            };

        if diff.deltas().len() > 0 {
            if let Some(msg) = commit.summary() {
                let msg = msg.trim();
                if !msg.is_empty() {
                    messages.push(msg.to_string());
                }
            }
        }
    }

    if messages.is_empty() {
        return None;
    }

    Some(format!("\n[git history]\n{}", messages.join("\n")))
}
