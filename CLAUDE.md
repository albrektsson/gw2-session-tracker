## Agent skills

### Issue tracker

Issues live in this repo's GitHub Issues (`albrektsson/gw2-session-tracker`), via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Conventions

- Default to zero inline comments. Don't add a comment just because a line
  does something non-trivial, or to explain a design/naming choice that's
  already clear from the code itself. Only write one when a future reader
  could not otherwise infer a genuinely non-obvious constraint or invariant
  — and even then, keep it to a single short line.
- Comments that do get written must describe the current state only. Don't
  narrate history — what an approach used to be, what was tried before, why
  a past attempt failed. That belongs in commit messages, not the file.
- Commit messages are a single line — a summary title, no body.
- Release notes are generated from the commit log by git-cliff
  (`cliff.toml`, run from `.github/workflows/release.yml`) rather than
  from PRs, since most commits land directly on `main`. Prefixing a
  subject with a type — `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`,
  `chore:`/`ci:` — groups it under that heading in the next release's
  notes instead of the catch-all "Other" section; see git-cliff's
  [conventional_commits strategy](https://git-cliff.org/docs/configuration/git/#conventional_commits).
  Not required — an unprefixed commit still shows up, just ungrouped.
