---
description: Full-text search past Claude Code sessions; show matches, answer from them, or give the resume command
---

Search my past Claude Code sessions for: $ARGUMENTS

Steps:

1. Refresh the index and search (both are near-instant):
   ```
   claude-recall index
   claude-recall list "$ARGUMENTS"
   ```
   Output columns (tab-separated): session_id, cwd, age, project, title.
   Strip ANSI codes when reading. Results are ranked best-match-first
   with a recency bonus.

2. Show the top ~5 matches as a readable list: age, project, title.
   For the top 2–3, include matched snippets via:
   ```
   claude-recall preview <session_id> "$ARGUMENTS"
   ```

3. Then, depending on what I asked for:
   - If I want information from a past session, read the full transcript at
     `~/.claude/projects/<project-dir>/<session_id>.jsonl` and answer from it.
   - If I want to continue a past session, note that a running session cannot
     resume another one — give me the exact command to run in a terminal:
     `cd <cwd> && claude --resume <session_id>`
     (or tell me to run `cs` and search there).

If `$ARGUMENTS` is empty, just list the most recent sessions.
