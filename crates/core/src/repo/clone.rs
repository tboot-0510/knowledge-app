//! Clone a GitHub repo by URL using the system `git` (shallow clone).

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Derive a filesystem-friendly repo name from a clone URL.
///
/// `https://github.com/owner/name.git` → `owner__name`
pub fn repo_name_from_url(url: &str) -> String {
    let trimmed = url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let parts: Vec<&str> = trimmed.rsplit(['/', ':']).take(2).collect();
    let name = match parts.as_slice() {
        [name, owner, ..] => format!("{owner}__{name}"),
        [name] => name.to_string(),
        _ => "repo".to_string(),
    };
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

/// Basic sanity check that a URL looks like a git remote we can clone.
pub fn is_valid_clone_url(url: &str) -> bool {
    let u = url.trim();
    u.starts_with("https://") || u.starts_with("http://") || u.starts_with("git@") || u.starts_with("ssh://")
}

/// Shallow-clone `url` into `dest_root/<derived-name>` and return the path.
/// If the destination already exists it is removed first (re-link/re-index).
pub fn clone_repo(url: &str, dest_root: &Path) -> Result<PathBuf> {
    if !is_valid_clone_url(url) {
        return Err(Error::Validation(format!("not a valid git URL: {url}")));
    }
    std::fs::create_dir_all(dest_root)?;
    let dest = dest_root.join(repo_name_from_url(url));
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    let output = Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&dest)
        .output()
        .map_err(|e| Error::Git(format!("failed to spawn git: {e}")))?;
    if !output.status.success() {
        return Err(Error::Git(String::from_utf8_lossy(&output.stderr).into_owned()));
    }
    Ok(dest)
}

/// Read the cloned repo's HEAD commit, if available.
pub fn head_commit(repo_path: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_derived_and_sanitized() {
        assert_eq!(repo_name_from_url("https://github.com/quiet-node/thuki.git"), "quiet-node__thuki");
        assert_eq!(repo_name_from_url("https://github.com/owner/name/"), "owner__name");
        assert_eq!(repo_name_from_url("git@github.com:owner/name.git"), "owner__name");
    }

    #[test]
    fn url_validation() {
        assert!(is_valid_clone_url("https://github.com/a/b"));
        assert!(is_valid_clone_url("git@github.com:a/b.git"));
        assert!(!is_valid_clone_url("not a url"));
        assert!(!is_valid_clone_url("/local/path"));
    }
}
