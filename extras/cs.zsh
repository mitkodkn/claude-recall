# Leave your shell in the resumed session's project directory.
#
# `cs` already runs `claude --resume` there, but it's a child process and
# a child can't move its parent — quit Claude and you're back where you
# started. This wrapper asks the picker where it went and follows it.
#
# Add to ~/.zshrc (works in bash too):
#   source /path/to/claude-recall/extras/cs.zsh
#
cs() {
  local cdfile dir rc
  cdfile="$(mktemp "${TMPDIR:-/tmp}/claude-recall-cd.XXXXXX")" || return 1
  CLAUDE_RECALL_CD_FILE="$cdfile" command cs "$@"
  rc=$?
  dir="$(cat "$cdfile" 2>/dev/null)"
  rm -f "$cdfile"
  # Empty unless a session was actually resumed (esc leaves you put).
  [ -n "$dir" ] && [ -d "$dir" ] && cd "$dir"
  return $rc
}
