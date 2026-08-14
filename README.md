# claude-recall

**Full-text search for your Claude Code session history — find any past conversation and resume it instantly.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![Powered by fzf](https://img.shields.io/badge/UI-fzf-green.svg)](https://github.com/junegunn/fzf)

Claude Code's built-in `claude --resume` picker only shows session titles.
`claude-recall` searches **everything you and Claude actually said** across
every project, ranks results by relevance and recency, and drops you back
into the conversation with one keystroke.

```
┌──────────────────────────────────────────────────┐
│ > stripe webhook█                                │
│ ▸ 2d ago  my-app       Fix Stripe webhook retr…  │
│   1w ago  boilerplate  Stripe balance transact…  │
│   3w ago  invoices     Parse Stripe payout rep…  │
│ ──────────────────────────────────────────────── │
│ [you] does stripe webhooks have a queue similar… │
│ [claude] …failed webhook jobs are retained in R… │
│                                                  │
│ ⏎ resume    ⌃F custom flags    esc quit          │
└──────────────────────────────────────────────────┘
```

## Features

- 🔍 **Full-text search of Claude Code chat history** — your prompts *and*
  Claude's replies, not just session titles
- ⚡ **Fast**: Rust + SQLite FTS5. ~6ms per keystroke as you type, ~20ms
  incremental re-index at launch, ~3s one-time full index of hundreds of
  sessions (about 1GB of history)
- 🎯 **Smart ranking**: BM25 relevance with a recency bonus — recent
  conversations surface first; the last word matches as a prefix while
  you're still typing
- 👀 **Live preview** of matched snippets, highlighted in context
- ↩️ **One-keystroke resume**: `Enter` jumps to the session's original
  project directory and runs `claude --resume`, with your preferred flags
- 🔒 **100% local and private**: no network calls, no telemetry — the index
  never leaves `~/.cache/claude-recall/`
- 🤖 Optional `/sessions` slash command to search your history **from
  inside Claude Code** and answer questions from past transcripts

## Install

Requires [fzf](https://github.com/junegunn/fzf), a Rust toolchain, zsh
(macOS default), and [Claude Code](https://claude.com/claude-code).

```sh
git clone https://github.com/<you>/claude-recall && cd claude-recall
cargo build --release
ln -s "$PWD/claude-recall.sh" ~/.local/bin/ccr   # or anywhere on PATH
```

Run `ccr`. The first run indexes your history in a few seconds; every run
after that opens instantly with your most recent sessions listed.

## Usage

| Key | Action |
|---|---|
| type | Full-text search across all sessions, ranked live |
| `↑` `↓` | Move through results (preview follows) |
| `Enter` | Resume the session in place, in its original project dir |
| `Ctrl-F` | Resume with custom `claude` flags for this run |
| `Esc` | Quit |

An empty query lists your most recent Claude Code sessions — so `ccr` ⏎ ⏎
is also the fastest way to reopen what you worked on last.

## Configuration

`claude-recall` adds no claude flags by default. To always resume with
certain flags, put them on one line in `~/.config/claude-recall/flags`:

```sh
mkdir -p ~/.config/claude-recall
echo "--dangerously-skip-permissions" > ~/.config/claude-recall/flags
```

(or set `$CLAUDE_RECALL_FLAGS`; the file wins). Active flags are shown in
the UI header. `$CLAUDE_RECALL_DB` overrides the index location.

## Search history from inside Claude Code

Copy `extras/sessions.md` to `~/.claude/commands/sessions.md` and put the
binary on PATH (`cargo install --path .`). Then, in any running session:

```
/sessions that redis queue bug we fixed
```

Claude searches your past sessions, shows matched snippets, answers
questions from the matched transcript, or prints the exact resume command.

**Bonus (macOS + Ghostty)**: add the keybind from `extras/ghostty-config`
for a system-wide `⌘⇧S` drop-down terminal — type `ccr` there for a
Spotlight-like session finder from anywhere.

## FAQ

**Where does Claude Code store session history?**
In `~/.claude/projects/<project>/<session-id>.jsonl` — one JSONL file per
session containing every message. `claude-recall` indexes user and
assistant text from these files, skipping tool output and internal noise.

**How is this different from `claude --resume`?**
The built-in picker lists sessions for the current project by title only.
`claude-recall` searches the full conversation content across **all**
projects, and still hands off to `claude --resume` for the actual resume.

**Does my conversation data go anywhere?**
No. Indexing and search are fully offline; the SQLite index stays on your
machine. Deleting `~/.cache/claude-recall/` removes it completely.

**Does it slow Claude Code down?**
No — it's a separate read-only tool. It never touches or modifies your
session files.

**Linux support?**
The Rust core is portable; the wrapper script needs zsh and was built on
macOS. PRs welcome.

## How it works

One Rust binary (`claude-recall`) with three subcommands: `index`
(incremental SQLite FTS5 indexing by file mtime/size), `list` (ranked TSV
for fzf), and `preview` (highlighted snippets). `claude-recall.sh` wires
them into fzf and handles the resume. ~400 lines total.

## License

[MIT](LICENSE)

---

*Keywords: Claude Code session search, search Claude Code chat history,
resume Claude Code conversation, Claude Code history viewer, claude
--resume alternative, Anthropic Claude CLI session manager, fzf, SQLite
FTS5, Rust.*
