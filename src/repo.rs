use std::process::Command;

use anyhow::{Result, bail};

/// Identifies the git repository containing the working directory: `owner/name`
/// from the `origin` remote, or the checkout's path when there is no origin.
pub fn current() -> Result<String> {
    if let Some(url) = git(&["remote", "get-url", "origin"]) {
        return Ok(name_from_remote(&url));
    }
    if let Some(toplevel) = git(&["rev-parse", "--show-toplevel"]) {
        return Ok(toplevel);
    }
    bail!("not inside a git repository; pass --repo <owner/name>")
}

/// Matches a stored repo against what a user typed: the full `owner/name`, or just `name`.
pub fn matches(stored: &str, wanted: &str) -> bool {
    let stored = stored.to_lowercase();
    let wanted = wanted.to_lowercase();
    stored == wanted || stored.ends_with(&format!("/{wanted}"))
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn name_from_remote(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/').map_or(rest, |(_, path)| path),
        None => match url.split_once(':') {
            Some((host, path)) if !host.contains('/') => path,
            _ => url,
        },
    };
    path.trim_start_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_urls_become_owner_and_name() {
        for url in [
            "git@github.com:limineol/papercuts.git",
            "https://github.com/limineol/papercuts.git",
            "https://github.com/limineol/papercuts",
            "ssh://git@github.com:22/limineol/papercuts.git",
        ] {
            assert_eq!(name_from_remote(url), "limineol/papercuts", "{url}");
        }
        assert_eq!(
            name_from_remote("https://gitlab.com/group/sub/project.git"),
            "group/sub/project"
        );
    }

    #[test]
    fn repo_matches_full_name_or_short_name() {
        assert!(matches("limineol/papercuts", "limineol/papercuts"));
        assert!(matches("limineol/papercuts", "Papercuts"));
        assert!(!matches("limineol/papercuts", "cuts"));
        assert!(!matches("limineol/papercuts", "limineol"));
    }
}
