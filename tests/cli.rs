use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root = std::env::temp_dir().join(format!("papercuts-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("work")).unwrap();
        Sandbox { root }
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    fn command(&self, cwd: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_papercuts"));
        command
            .args(args)
            .current_dir(cwd)
            .env("PAPERCUTS_HOME", self.root.join("store"))
            .env("GIT_CEILING_DIRECTORIES", &self.root);
        command
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(cwd, args).output().unwrap()
    }

    fn json(&self, cwd: &Path, args: &[&str]) -> Value {
        let output = self.run(cwd, args);
        assert!(
            output.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn add(&self, summary: &str, extra: &[&str]) -> Value {
        let mut args = vec!["add", summary, "--json"];
        args.extend_from_slice(extra);
        self.json(&self.work(), &args)
    }

    fn ids(&self, filters: &[&str]) -> Vec<u64> {
        let mut args = vec!["list", "--json"];
        args.extend_from_slice(filters);
        self.json(&self.work(), &args)
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_u64().unwrap())
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn recorded_papercut_can_be_retrieved_with_every_field() {
    let sandbox = Sandbox::new("round-trip");
    let recorded = sandbox.add(
        "Claimed the tests passed without running them",
        &[
            "--severity",
            "high",
            "--cause",
            "verification",
            "--fix-in",
            "repo AGENTS.md",
            "--scope",
            "codebase",
            "--repo",
            "acme/widgets",
            "--details",
            "User: \"you didn't even run them\"",
            "--at",
            "2026-11-10T14:30:00Z",
        ],
    );
    assert_eq!(
        recorded,
        json!({
            "id": 1,
            "summary": "Claimed the tests passed without running them",
            "severity": "high",
            "cause": "verification",
            "fix_in": "repo AGENTS.md",
            "scope": "codebase",
            "repo": "acme/widgets",
            "details": "User: \"you didn't even run them\"",
            "occurred_at": "2026-11-10T14:30:00Z",
        })
    );

    assert_eq!(
        sandbox.json(&sandbox.work(), &["show", "1", "--json"]),
        recorded
    );
    assert_eq!(
        sandbox.json(&sandbox.work(), &["list", "--json"]),
        json!([recorded])
    );

    let missing = sandbox.run(&sandbox.work(), &["show", "2"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no papercut with id 2"));
}

#[test]
fn codebase_papercuts_default_to_the_current_repository() {
    let sandbox = Sandbox::new("repo");
    let repo = sandbox.work().join("widgets");
    fs::create_dir_all(repo.join("src")).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&repo)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q"]);
    git(&["remote", "add", "origin", "git@github.com:acme/widgets.git"]);

    let base = [
        "--severity",
        "low",
        "--cause",
        "context",
        "--fix-in",
        "docs",
    ];
    let mut args = vec!["add", "In the repo", "--json", "--scope", "codebase"];
    args.extend_from_slice(&base);
    let recorded = sandbox.json(&repo.join("src"), &args);
    assert_eq!(recorded["repo"], "acme/widgets");

    let mut args = vec!["add", "Global", "--json", "--scope", "global"];
    args.extend_from_slice(&base);
    let global = sandbox.json(&repo, &args);
    assert_eq!(global["repo"], Value::Null);

    let here = sandbox.json(&repo, &["list", "--json", "--here"]);
    assert_eq!(here, json!([recorded]));
    assert_eq!(sandbox.ids(&["--repo", "widgets"]), [1]);
    assert_eq!(sandbox.ids(&["--repo", "ACME/Widgets"]), [1]);
    assert_eq!(sandbox.ids(&["--repo", "gadgets"]), Vec::<u64>::new());

    git(&["remote", "set-url", "origin", "git@selfhosted:widgets.git"]);
    let here = sandbox.json(&repo, &["list", "--json", "--here"]);
    assert_eq!(here, json!([]), "--here matched another owner's widgets");

    let mut args = vec!["add", "Outside a repo", "--scope", "codebase"];
    args.extend_from_slice(&base);
    let outside = sandbox.run(&sandbox.work(), &args);
    assert!(!outside.status.success());
    assert!(String::from_utf8_lossy(&outside.stderr).contains("pass --repo"));

    let mut args = vec!["add", "Contradictory", "--scope", "global", "--repo", "x"];
    args.extend_from_slice(&base);
    assert!(!sandbox.run(&repo, &args).status.success());
}

#[test]
fn list_filters_and_shows_newest_first() {
    let sandbox = Sandbox::new("filters");
    assert_eq!(sandbox.ids(&[]), Vec::<u64>::new());

    for (summary, severity, cause, scope, at) in [
        (
            "old",
            "high",
            "instructions",
            "global",
            "2026-01-05T12:00:00Z",
        ),
        ("new", "low", "skill", "global", "2026-11-05T12:00:00Z"),
        (
            "mid",
            "medium",
            "instructions",
            "codebase",
            "2026-06-05T12:00:00Z",
        ),
    ] {
        let mut args = vec![
            "--severity",
            severity,
            "--cause",
            cause,
            "--fix-in",
            "AGENTS.md",
            "--scope",
            scope,
            "--at",
            at,
        ];
        if scope == "codebase" {
            args.extend(["--repo", "acme/widgets"]);
        }
        sandbox.add(summary, &args);
    }

    assert_eq!(sandbox.ids(&[]), [2, 3, 1]);
    assert_eq!(sandbox.ids(&["--limit", "1"]), [2]);
    assert_eq!(sandbox.ids(&["--min-severity", "medium"]), [3, 1]);
    assert_eq!(sandbox.ids(&["--cause", "instructions"]), [3, 1]);
    assert_eq!(sandbox.ids(&["--scope", "global"]), [2, 1]);
    assert_eq!(sandbox.ids(&["--since", "2026-06-01"]), [2, 3]);
    assert_eq!(sandbox.ids(&["--since", "2026-11-05T12:00:00Z"]), [2]);
}

#[test]
fn concurrent_agents_get_distinct_ids() {
    let sandbox = Sandbox::new("concurrent");
    let children: Vec<_> = (0..16)
        .map(|n| {
            let summary = format!("papercut {n}");
            sandbox
                .command(
                    &sandbox.work(),
                    &[
                        "add",
                        &summary,
                        "--severity",
                        "low",
                        "--cause",
                        "other",
                        "--fix-in",
                        "nowhere",
                        "--scope",
                        "global",
                    ],
                )
                .stdout(Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }

    let mut ids = sandbox.ids(&[]);
    ids.sort();
    assert_eq!(ids, (1..=16).collect::<Vec<u64>>());
}
