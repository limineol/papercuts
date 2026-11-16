use clap::ValueEnum;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Papercut {
    pub id: u64,
    pub summary: String,
    pub severity: Severity,
    pub cause: Cause,
    pub fix_in: String,
    pub scope: Scope,
    /// Set exactly when `scope` is `Codebase`.
    pub repo: Option<String>,
    pub details: Option<String>,
    pub occurred_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Minor annoyance, corrected quickly
    Low,
    /// Cost the user real time or a redo
    Medium,
    /// Broke trust: a false claim, lost work, or a repeat of something already corrected
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Cause {
    /// AGENTS.md, CLAUDE.md or other rules are missing, unclear or wrong
    Instructions,
    /// The agent had no way to check its own work (tests, run scripts, verification skills)
    Verification,
    /// A skill is missing, wrong, or did not trigger
    Skill,
    /// Knowledge about the codebase or domain is missing or undocumented
    Context,
    /// A harness, tool, permission or environment limitation
    Tooling,
    /// The agent had what it needed and still got it wrong
    Judgment,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    /// Specific to one codebase
    Codebase,
    /// Applies across codebases: agent behavior, global instructions, skills, the harness
    Global,
}

pub fn label(value: &impl ValueEnum) -> String {
    value
        .to_possible_value()
        .expect("no variant is skipped")
        .get_name()
        .to_string()
}
