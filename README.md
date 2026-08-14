# claude-sessions

Spotlight-style **full-text search over your Claude Code session history**,
with one-keystroke resume. Claude Code's built-in `--resume` picker only
shows session titles; this searches everything you and Claude actually said,
ranked by relevance and recency, and drops you back into the session.

```
┌──────────────────────────────────────────────────┐
│ > stripe webhook█                                │
│ ▸ 2d ago  my-app       Fix Stripe webhook retr…  │
│   1w ago  boilerplate  Stripe balance transact…  │
│ ──────────────────────────────────────────────── │
│ [you] does stripe webhooks have a queue similar… │
│ [claude] …failed webhook jobs are retained in R… │
└──────────────────────────────────────────────────┘
```

- **Fast**: Rust + SQLite FTS5. ~6ms per keystroke, ~20ms incremental
  reindex on launch, ~3s one-time full index of hundreds of sessions.
- **Private by design**: everything stays on your machine. The index
  lives in `~/.cache/claude-sessions/`.

## Requirements

- [fzf](https://github.com/junegunn/fzf) ≥ 0.38
- Rust toolchain (to build)
- zsh (the wrapper script; default shell on macOS)
- [Claude Code](https://claude.com/claude-code) with sessions in
  `~/.claude/projects/`

## Install

```sh
git clone <this-repo> && cd claude-sessions
cargo build --release
ln -s "$PWD/claude-sessions.sh" ~/.local/bin/cs   # or anywhere on PATH
```

Run `cs` in a terminal. First run builds the index; after that it opens
instantly with your most recent sessions listed.

## Usage

- Empty query lists the most recent sessions; typing full-text searches
  **all chat content**, ranked by BM25 match quality with a recency
  bonus; the last word matches as a prefix.
- `↑`/`↓` to move; the bottom pane previews matched snippets.
- `Enter` — cd to the session's project dir and `claude --resume` it,
  with your configured default flags (see below).
- `Ctrl-F` — prompt for custom flags for this resume instead.
- `Esc` — quit.

## Configuration

The tool adds no claude flags by default. To always resume with certain
flags, put them on one line in `~/.config/claude-sessions/flags`, e.g.:

```sh
mkdir -p ~/.config/claude-sessions
echo "--dangerously-skip-permissions" > ~/.config/claude-sessions/flags
```

(or set `$CLAUDE_SESSIONS_FLAGS`; the file wins). Active flags are shown
in the UI header. `$CLAUDE_SESSIONS_DB` overrides the index location.

## Optional extras

- **Search from inside Claude Code**: copy `extras/sessions.md` to
  `~/.claude/commands/sessions.md` (needs the binary on PATH:
  `cargo install --path .`). Then `/sessions <query>` in any session
  searches your history, shows snippets, and can answer questions from
  a matched transcript.
- **Global hotkey (macOS + Ghostty)**: add the keybind from
  `extras/ghostty-config` to your Ghostty config — `⌘⇧S` toggles a
  drop-down quick terminal anywhere in macOS; type `cs` there.

## How it works

One Rust binary, `claude-sessions-core`, with three subcommands:

- `index` — walks `~/.claude/projects/*/*.jsonl`, extracts user and
  assistant message text (skipping tool output, subagent sidechains,
  and harness meta-messages) into SQLite FTS5. Incremental by file
  mtime/size; runs synchronously on every launch.
- `list [query]` — TSV rows for fzf, ranked; empty query = recent.
- `preview SID [query]` — highlighted matched snippets for the
  preview pane.

`claude-sessions.sh` wires those into fzf and handles the resume
(cd to the session's original cwd, then `claude --resume <id>`).

## License

MIT
