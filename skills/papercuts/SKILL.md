---
name: papercuts
description: Use when the user complains about or corrects something an agent did or failed to do ("why did you…", "I already told you…", "that's wrong again", visible frustration). Record a papercut incident report with the `papercuts` CLI after addressing the complaint. Also use when asked to review, triage, or fix recorded papercuts.
---

# Papercuts

A papercut is a small, avoidable friction between the user and their agents.
Recording each one with its cause and the place its fix belongs turns
complaints into a backlog of fixes for instructions, skills, codebases, and
tooling. Requires the `papercuts` CLI on `PATH`.

## When to record

Record one papercut per distinct complaint about agent behavior: a false
claim, an ignored instruction, missing verification, a wrong assumption, a
convention broken again. Do not record ordinary feature requests, changed
requirements, or product bugs the agent did not cause.

Address the complaint first, then record it in the same turn so it is not
lost. Tell the user in one line, such as "Logged papercut 12."

## How to record

```sh
papercuts add "<what went wrong, one line, from the user's point of view>" \
  --severity <low|medium|high> \
  --cause <instructions|verification|skill|context|tooling|judgment|other> \
  --scope <codebase|global> \
  --fix-in "<where the fix most likely belongs>" \
  --details "<what happened, and the user's words>"
```

- **severity**: `low` for a quick correction, `medium` when it cost real time
  or a redo, `high` when it broke trust (a false claim, lost work, or a repeat
  of something already corrected).
- **cause**: ask what would have prevented it, and pick the earliest missing
  piece. `instructions`: an AGENTS.md or rule was missing, unclear, or wrong.
  `verification`: there was no way to check the work (tests, run scripts,
  verification skills). `skill`: a skill was missing, wrong, or did not
  trigger. `context`: knowledge about the codebase or domain was missing.
  `tooling`: a harness, tool, permission, or environment limit. `judgment`:
  everything needed was available and the agent still got it wrong.
- **scope**: `codebase` when the fix belongs in the repository (its AGENTS.md,
  docs, scripts, tests); `global` when it belongs in the agent's own
  instructions, skills, or harness. `codebase` records the current git
  repository; pass `--repo owner/name` when working outside it.
- **fix-in**: name a concrete place: a file path, a skill, a script to add, a
  setting. "Be more careful" is not a place.
- **details**: quote the user and note what you did, so the fix can be made
  without this conversation.

## Retrieving

```sh
papercuts list --here                  # this repository's papercuts
papercuts list --scope global --min-severity medium
papercuts list --cause verification --since 2026-11-01
papercuts show <id>
```

Add `--json` for machine-readable output. When asked to fix papercuts, group
them by `fix_in`, fix each at that place, and say which ids each change covers.
