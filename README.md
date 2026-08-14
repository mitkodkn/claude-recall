# claude-recall

![claude-recall — a crab librarian searching an archive of terminal drawers](assets/banner.png)

**Full-text search for your Claude Code session history — because that
genius fix from three weeks ago is in there *somewhere*.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![Powered by fzf](https://img.shields.io/badge/UI-fzf-green.svg)](https://github.com/junegunn/fzf)

You've had hundreds of conversations with Claude Code. Together you fixed
that webhook bug, untangled that Redis queue, wrote that migration you're
still a little proud of. Claude remembers none of it, and honestly,
neither do you.

The built-in `claude --resume` picker shows you a list of titles like a
witness lineup. Was it *"fix the thing"*? *"handle tickets"*? *"asdf"*?
Good luck.

`claude-recall` searches **everything you and Claude actually said**,
across every project, ranked by relevance and recency — and drops you
back into the conversation with one keystroke. It's `⌘F` for your past
selves.

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

## What you get

- 🔍 **Real full-text search** of your Claude Code chat history — your
  prompts *and* Claude's replies, not just the titles
- ⚡ **Stupid fast**: Rust + SQLite FTS5. ~6ms per keystroke, ~20ms
  re-index at launch, ~3s to index a gigabyte of history once. The
  bottleneck is your typing.
- 🎯 **Ranking that gets it**: BM25 relevance with a recency bonus, and
  the last word matches as a prefix — results are right before you
  finish typ
- 👀 **Live preview** of the matched snippets, highlighted, so you can
  tell your five "stripe webhook" sessions apart
- ↩️ **One-keystroke resume**: `Enter` jumps to the session's original
  project directory and runs `claude --resume` with your preferred flags
- 🔒 **Local. Period.** No network calls, no telemetry, no "sign in to
  continue". The index lives in `~/.cache/claude-recall/` and can be
  deleted whenever you want
- 🤖 A `/sessions` slash command so Claude itself can dig through your
  shared past — *"what did we decide about the retry logic?"* — from
  inside a running session

## Install

You'll need [fzf](https://github.com/junegunn/fzf), a Rust toolchain,
zsh (macOS default), and [Claude Code](https://claude.com/claude-code)
with some history worth finding.

```sh
git clone https://github.com/<you>/claude-recall && cd claude-recall
cargo build --release
ln -s "$PWD/claude-recall.sh" ~/.local/bin/cs   # or anywhere on PATH
```

The command is `cs` — short for **c**laude **s**earch. First run indexes
everything in a few seconds. Every run after that opens instantly with
your most recent sessions on top — so `cs` ⏎ is also the fastest
"reopen what I was doing yesterday" there is.

## Driving it

| Key | Action |
|---|---|
| type | Full-text search across all sessions, live |
| `↑` `↓` | Move through results, preview follows |
| `Enter` | Resume the session, right there in your terminal |
| `Ctrl-F` | Resume with custom `claude` flags, this once |
| `Esc` | Return to the present |

## Configuration

By default, resume adds no flags. If you always want some (you know the
one), put them on a line in `~/.config/claude-recall/flags`:

```sh
mkdir -p ~/.config/claude-recall
echo "--dangerously-skip-permissions" > ~/.config/claude-recall/flags
```

(or set `$CLAUDE_RECALL_FLAGS`; the file wins). The active flags are shown
in the UI header so `Enter` never surprises you. `$CLAUDE_RECALL_DB`
overrides where the index lives.

## Let Claude search its own past

Copy `extras/sessions.md` to `~/.claude/commands/sessions.md` and put the
binary on PATH (`cargo install --path .`). Then, mid-session:

```
/sessions that redis queue bug we fixed
```

Claude searches your history, shows the matches, and either answers from
the old transcript or hands you the exact resume command. Yes, it's
Claude reading Claude's diary. It's fine. Everyone's fine.

**Bonus (macOS + Ghostty)**: the keybind in `extras/ghostty-config` gives
you a system-wide `⌘⇧S` drop-down terminal — type `cs` there and it's
basically Spotlight for your Claude sessions.

## FAQ

**Where does Claude Code store session history?**
`~/.claude/projects/<project>/<session-id>.jsonl` — one JSONL file per
session, every message included. `claude-recall` indexes the user and
assistant text and skips the tool spam, subagent chatter, and other
robot noises.

**How is this different from `claude --resume`?**
The built-in picker lists the current project's sessions by title.
`claude-recall` searches full conversation content across **all**
projects — then uses `claude --resume` for the actual resume. Friends,
not rivals.

**Does my conversation data go anywhere?**
No. Indexing and search are fully offline. Delete
`~/.cache/claude-recall/` and it's like nothing ever happened.

**Will it slow Claude Code down?**
No — it's a separate, read-only tool. It never modifies your session
files. It just reads. Like a very fast librarian.

**Linux?**
The Rust core is portable; the wrapper wants zsh and grew up on macOS.
PRs welcome.

## How it works

One Rust binary (`claude-recall`) with three subcommands — `index`
(incremental SQLite FTS5, keyed on file mtime/size), `list` (ranked TSV
for fzf), `preview` (highlighted snippets) — and one zsh script that
wires them into fzf and handles the resume. ~400 lines total. No
framework survived contact with this project.

## License

[MIT](LICENSE)

---

*Keywords: Claude Code session search, search Claude Code chat history,
resume Claude Code conversation, Claude Code history viewer, claude
--resume alternative, Anthropic Claude CLI session manager, fzf, SQLite
FTS5, Rust.*
