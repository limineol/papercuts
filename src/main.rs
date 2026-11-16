mod papercut;
mod repo;
mod store;

use std::cmp::Reverse;
use std::io::{self, ErrorKind, Write};
use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Unit};

use papercut::{Cause, Papercut, Scope, Severity, label};
use store::Store;

/// Papercut incident reports for coding agents.
///
/// When a user complains about something an agent did, the agent records a
/// papercut: what went wrong, how severe it was, where the problem comes from,
/// and where the fix most likely belongs. Reports are stored in
/// $PAPERCUTS_HOME/papercuts.jsonl (default: $XDG_DATA_HOME/papercuts, else
/// ~/.local/share/papercuts).
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Record a papercut
    Add(AddArgs),
    /// List papercuts, newest first
    List(ListArgs),
    /// Show one papercut in full
    Show {
        id: u64,
        /// Print JSON instead of text
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args)]
struct AddArgs {
    /// What went wrong, in one line, from the user's point of view
    #[arg(value_parser = non_empty)]
    summary: String,
    #[arg(long, value_enum)]
    severity: Severity,
    /// Where the problem comes from
    #[arg(long, value_enum)]
    cause: Cause,
    /// Where the fix most likely belongs, e.g. "repo AGENTS.md" or "~/.agents/skills/prepare-pr"
    #[arg(long, value_parser = non_empty)]
    fix_in: String,
    /// Whether the papercut is specific to one codebase
    #[arg(long, value_enum)]
    scope: Scope,
    /// The codebase, for --scope codebase [default: the current git repository]
    #[arg(long, value_parser = non_empty)]
    repo: Option<String>,
    /// What happened, what the user said, and anything else worth keeping
    #[arg(long)]
    details: Option<String>,
    /// When it happened, as an RFC 3339 timestamp [default: now]
    #[arg(long)]
    at: Option<Timestamp>,
    /// Print the recorded papercut as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct ListArgs {
    /// Only codebase papercuts for this repo (owner/name, or just name)
    #[arg(long, conflicts_with = "here")]
    repo: Option<String>,
    /// Only codebase papercuts for the current git repository
    #[arg(long)]
    here: bool,
    #[arg(long, value_enum)]
    scope: Option<Scope>,
    /// Only papercuts at least this severe
    #[arg(long, value_enum)]
    min_severity: Option<Severity>,
    #[arg(long, value_enum)]
    cause: Option<Cause>,
    /// Only papercuts from this date (YYYY-MM-DD, local time) or RFC 3339 timestamp onward
    #[arg(long, value_parser = parse_since)]
    since: Option<Timestamp>,
    /// Show only the newest N
    #[arg(long)]
    limit: Option<usize>,
    /// Print JSON instead of a table
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err)
            if err
                .downcast_ref::<io::Error>()
                .is_some_and(|err| err.kind() == ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let store = Store::open()?;
    let mut out = io::stdout().lock();
    match cli.command {
        Command::Add(args) => add(&store, args, &mut out),
        Command::List(args) => list(&store, args, &mut out),
        Command::Show { id, json } => {
            let Some(papercut) = store.load()?.into_iter().find(|p| p.id == id) else {
                bail!("no papercut with id {id}");
            };
            if json {
                writeln!(out, "{}", serde_json::to_string_pretty(&papercut)?)?;
            } else {
                print_report(&papercut, &mut out)?;
            }
            Ok(())
        }
    }
}

fn add(store: &Store, args: AddArgs, out: &mut impl Write) -> Result<()> {
    let repo = match (args.scope, args.repo) {
        (Scope::Codebase, Some(repo)) => Some(repo),
        (Scope::Codebase, None) => Some(repo::current()?),
        (Scope::Global, Some(_)) => bail!("--repo only applies to --scope codebase"),
        (Scope::Global, None) => None,
    };
    let occurred_at = match args.at {
        Some(at) => at,
        None => Timestamp::now().round(Unit::Second)?,
    };
    let papercut = store.append(|id| Papercut {
        id,
        summary: args.summary,
        severity: args.severity,
        cause: args.cause,
        fix_in: args.fix_in,
        scope: args.scope,
        repo,
        details: args.details,
        occurred_at,
    })?;
    if args.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&papercut)?)?;
    } else {
        writeln!(out, "Recorded papercut {}.", papercut.id)?;
    }
    Ok(())
}

fn list(store: &Store, args: ListArgs, out: &mut impl Write) -> Result<()> {
    let here = if args.here {
        Some(repo::current()?)
    } else {
        None
    };
    let mut papercuts: Vec<Papercut> = store
        .load()?
        .into_iter()
        .filter(|p| {
            let stored = p.repo.as_deref();
            args.repo
                .as_ref()
                .is_none_or(|wanted| stored.is_some_and(|r| repo::matches(r, wanted)))
                && here
                    .as_ref()
                    .is_none_or(|current| stored.is_some_and(|r| r.eq_ignore_ascii_case(current)))
                && args.scope.is_none_or(|scope| p.scope == scope)
                && args.min_severity.is_none_or(|min| p.severity >= min)
                && args.cause.is_none_or(|cause| p.cause == cause)
                && args.since.is_none_or(|since| p.occurred_at >= since)
        })
        .collect();
    papercuts.sort_by_key(|p| Reverse((p.occurred_at, p.id)));
    if let Some(limit) = args.limit {
        papercuts.truncate(limit);
    }

    if args.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&papercuts)?)?;
    } else if papercuts.is_empty() {
        writeln!(out, "No papercuts found.")?;
    } else {
        print_table(&papercuts, out)?;
    }
    Ok(())
}

fn print_table(papercuts: &[Papercut], out: &mut impl Write) -> Result<()> {
    let width = papercuts
        .iter()
        .map(|p| location(p).len())
        .fold("WHERE".len(), usize::max);
    writeln!(
        out,
        "{:>4}  {:<16}  {:<8}  {:<12}  {:<width$}  SUMMARY",
        "ID", "WHEN", "SEVERITY", "CAUSE", "WHERE"
    )?;
    for papercut in papercuts {
        writeln!(
            out,
            "{:>4}  {:<16}  {:<8}  {:<12}  {:<width$}  {}",
            papercut.id,
            local_time(papercut.occurred_at, "%Y-%m-%d %H:%M"),
            label(&papercut.severity),
            label(&papercut.cause),
            location(papercut),
            papercut.summary,
        )?;
    }
    Ok(())
}

fn print_report(papercut: &Papercut, out: &mut impl Write) -> Result<()> {
    writeln!(out, "Papercut {}: {}", papercut.id, papercut.summary)?;
    writeln!(out, "  Severity  {}", label(&papercut.severity))?;
    writeln!(out, "  Cause     {}", label(&papercut.cause))?;
    writeln!(out, "  Fix in    {}", papercut.fix_in)?;
    writeln!(out, "  Where     {}", location(papercut))?;
    writeln!(
        out,
        "  When      {}",
        local_time(papercut.occurred_at, "%Y-%m-%d %H:%M %:z")
    )?;
    if let Some(details) = &papercut.details {
        writeln!(out, "\n{details}")?;
    }
    Ok(())
}

fn location(papercut: &Papercut) -> &str {
    papercut.repo.as_deref().unwrap_or("global")
}

fn local_time(timestamp: Timestamp, format: &str) -> String {
    timestamp
        .to_zoned_datetime(TimeZone::system())
        .strftime(format)
        .to_string()
}

fn non_empty(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("must not be empty".to_string());
    }
    Ok(value.to_string())
}

fn parse_since(value: &str) -> Result<Timestamp, String> {
    if let Ok(timestamp) = value.parse::<Timestamp>() {
        return Ok(timestamp);
    }
    let date: Date = value
        .parse()
        .map_err(|_| "expected YYYY-MM-DD or an RFC 3339 timestamp".to_string())?;
    date.to_zoned_datetime(TimeZone::system())
        .map(|start| start.timestamp())
        .map_err(|err| err.to_string())
}
