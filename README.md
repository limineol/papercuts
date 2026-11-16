# papercuts

Papercut incident reports for coding agents.

When you complain about something an agent did (it claimed the tests passed
without running them, opened a draft PR you didn't ask for, ignored a
convention you already explained), the agent records a **papercut**: what went
wrong, how severe it was, where the problem comes from, and where it is most
likely to be fixed. Later, you or an agent pull up the papercuts and fix them
at the source: an `AGENTS.md`, a skill, a missing test script, a harness
setting.

## Install

```sh
cargo install --locked --git https://github.com/limineol/papercuts
```

Requires Rust 1.89 or newer.

## Use

```sh
papercuts add "Claimed the tests passed without running them" \
  --severity high \
  --cause verification \
  --scope codebase \
  --fix-in "repo AGENTS.md: document how to run the test suite" \
  --details "User: 'you didn't even run them.'"

papercuts list                     # everything, newest first
papercuts list --here              # papercuts for the current repository
papercuts list --scope global --min-severity medium --since 2026-11-01
papercuts show 12
```

Add `--json` to any command for machine-readable output. `papercuts <command>
--help` documents every option.

## What a papercut records

| Field         | Meaning                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------- |
| `summary`     | What went wrong, in one line, from the user's point of view                                             |
| `severity`    | `low` (corrected quickly), `medium` (cost real time or a redo), `high` (broke trust)                    |
| `cause`       | Where it comes from: `instructions`, `verification`, `skill`, `context`, `tooling`, `judgment`, `other` |
| `fix_in`      | Where the fix most likely belongs, such as a file, a skill, or a script to add                          |
| `scope`       | `codebase` if it is specific to one repository, `global` if it applies everywhere                       |
| `repo`        | The repository, for codebase papercuts: `owner/name` from the `origin` remote by default                |
| `details`     | What happened and what the user said                                                                    |
| `occurred_at` | When it happened; defaults to the moment it was recorded                                                |

## Storage

Papercuts live in one JSON Lines file, `papercuts.jsonl`, in `$PAPERCUTS_HOME`
(default: `$XDG_DATA_HOME/papercuts`, else `~/.local/share/papercuts`). Each
write holds a file lock, so agents running in parallel get distinct, sequential
ids.

## Agent setup

Agents have to know when to record a papercut. Copy
[`skills/papercuts`](skills/papercuts/SKILL.md) into your harness's skills
directory (for example `~/.claude/skills/` or `~/.agents/skills/`), or add a
line like this to your global `AGENTS.md`:

> When I complain about something you did, address it, then record it with
> `papercuts add` (see `papercuts add --help`).

## Why a CLI and not an MCP server

Every coding agent already has a shell, so a CLI works in any harness without
running a server or configuring each tool. It is also easy for people to use
and script. The skill supplies what an MCP tool description would: when to
record a papercut and how to fill it in.

## License

[MIT](LICENSE)
