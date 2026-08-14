#!/bin/zsh
# Spotlight-style Claude Code session finder. Runs inside a terminal;
# Enter resumes the selected session in place via `claude --resume`.
set -u
DIR="${0:A:h}"
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
BIN="$DIR/target/release/claude-recall"
if [[ ! -x "$BIN" ]]; then
  BIN="$(command -v claude-recall)" || {
    print "claude-recall not found — build it with:"
    print "  cargo build --release   (in the repo)  or  cargo install --path ."
    exit 1
  }
fi

# Default claude flags for resume come from the user's setup, not the tool:
# ~/.config/claude-recall/flags (one line), or $CLAUDE_RECALL_FLAGS.
flags="${CLAUDE_RECALL_FLAGS:-}"
conf="$HOME/.config/claude-recall/flags"
[[ -f "$conf" ]] && flags="$(head -1 "$conf")"

header='⏎ resume    ⌃F custom flags    esc quit'
[[ -n "$flags" ]] && header="⏎ resume ($flags)    ⌃F custom flags    esc quit"

"$BIN" index 2>/dev/null   # incremental: ~20ms, so run synchronously

out=$("$BIN" list '' | fzf \
  --ansi --disabled --no-multi \
  --prompt 'claude ❯ ' \
  --header "$header" \
  --header-first \
  --delimiter '\t' --with-nth 3,4,5 \
  --bind "change:reload:$BIN list {q}" \
  --preview "$BIN preview {1} {q}" \
  --preview-window 'down,40%,wrap,border-top' \
  --expect=ctrl-f \
  --no-info --layout=reverse --border=rounded \
  --color 'prompt:cyan,pointer:cyan,header:dim')

[[ "$out" != *$'\n'* ]] && exit 0   # aborted or nothing selected
key="${out%%$'\n'*}"
sel="${out#*$'\n'}"
[[ -z "$sel" ]] && exit 0

sid="${sel%%$'\t'*}"
rest="${sel#*$'\t'}"
cwd="${rest%%$'\t'*}"
[[ -z "$sid" ]] && exit 0

if [[ "$key" == "ctrl-f" ]]; then
  print -n "claude flags [default: ${flags:-none}]: "
  read -r custom
  [[ -n "$custom" ]] && flags="$custom"
fi

cd "$cwd" 2>/dev/null || cd "$HOME"
claude --resume "$sid" ${=flags}
