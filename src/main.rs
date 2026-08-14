// Fast core for the Claude Code session finder.
//   claude-sessions-core index            incrementally (re)index sessions
//   claude-sessions-core list [query]     TSV rows for fzf
//   claude-sessions-core preview SID [q]  human-readable session preview

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const MAX_MSG_CHARS: usize = 8000;
const TITLE_CHARS: usize = 100;
const LIMIT: usize = 60;

const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS files(
  path TEXT PRIMARY KEY, mtime REAL, size INTEGER, session_id TEXT);
CREATE TABLE IF NOT EXISTS sessions(
  session_id TEXT PRIMARY KEY, project TEXT, cwd TEXT,
  title TEXT, first_ts TEXT, last_ts TEXT, msgs INTEGER);
CREATE VIRTUAL TABLE IF NOT EXISTS msg_fts USING fts5(
  text, session_id UNINDEXED, role UNINDEXED, ts UNINDEXED,
  tokenize='unicode61');
";

const NOISE: [&str; 6] = [
    "<command-name>", "<command-message>", "<command-args>",
    "<local-command-stdout>", "<system-reminder>", "<task-notification>",
];

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME not set"))
}

fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("CLAUDE_SESSIONS_DB") {
        return PathBuf::from(p);
    }
    home().join(".cache/claude-sessions/index.db")
}

fn open_db() -> Connection {
    let path = db_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let con = Connection::open(&path).expect("open index db");
    con.execute_batch(SCHEMA).expect("schema");
    con
}

fn truncate_chars(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

fn extract_text(message: &Value) -> String {
    match message.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn is_noise(text: &str) -> bool {
    let head = truncate_chars(text, 200);
    NOISE.iter().any(|n| head.contains(n))
}

fn parse_ts(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts).ok().map(|t| t.with_timezone(&Utc))
}

fn age_str(ts: &str) -> String {
    let Some(then) = parse_ts(ts) else { return "?".into() };
    let secs = (Utc::now() - then).num_seconds().max(0);
    match secs {
        s if s < 3600 => format!("{}m", (s / 60).max(1)),
        s if s < 86_400 => format!("{}h", s / 3600),
        s if s < 604_800 => format!("{}d", s / 86_400),
        s if s < 2_592_000 => format!("{}w", s / 604_800),
        s if s < 31_536_000 => format!("{}mo", s / 2_592_000),
        s => format!("{}y", s / 31_536_000),
    }
}

fn age_days(ts: &str) -> f64 {
    parse_ts(ts)
        .map(|t| (Utc::now() - t).num_seconds().max(0) as f64 / 86_400.0)
        .unwrap_or(9999.0)
}

fn fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let mut parts: Vec<String> =
        tokens.iter().map(|t| format!("\"{t}\"")).collect();
    let last = parts.pop().unwrap();
    parts.push(format!("{last}*"));
    Some(parts.join(" "))
}

// ---------------- index ----------------

fn index_file(con: &Connection, path: &str) -> usize {
    let session_id = PathBuf::from(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut cwd = String::new();
    let mut title = String::new();
    let mut first_ts: Option<String> = None;
    let mut last_ts: Option<String> = None;
    let mut rows: Vec<(String, String, String)> = Vec::new(); // text, role, ts

    let Ok(fh) = std::fs::File::open(path) else { return 0 };
    for line in BufReader::new(fh).lines().map_while(Result::ok) {
        let Ok(rec) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(c) = rec.get("cwd").and_then(Value::as_str) {
            if !c.is_empty() {
                cwd = c.to_string();
            }
        }
        let rtype = rec.get("type").and_then(Value::as_str).unwrap_or("");
        if rtype != "user" && rtype != "assistant" {
            continue;
        }
        if rec.get("isSidechain").and_then(Value::as_bool).unwrap_or(false)
            || rec.get("isMeta").and_then(Value::as_bool).unwrap_or(false)
        {
            continue;
        }
        let text = extract_text(rec.get("message").unwrap_or(&Value::Null));
        if text.trim().is_empty() || is_noise(&text) {
            continue;
        }
        let ts = rec
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if first_ts.is_none() {
            first_ts = Some(ts.clone());
        }
        if !ts.is_empty() {
            last_ts = Some(ts.clone());
        }
        if title.is_empty() && rtype == "user" {
            let origin = rec
                .get("origin")
                .and_then(|o| o.get("kind"))
                .and_then(Value::as_str);
            if matches!(origin, None | Some("human")) && !text.starts_with('<') {
                let joined =
                    text.split_whitespace().collect::<Vec<_>>().join(" ");
                title = truncate_chars(&joined, TITLE_CHARS).to_string();
            }
        }
        rows.push((
            truncate_chars(&text, MAX_MSG_CHARS).to_string(),
            rtype.to_string(),
            ts,
        ));
    }

    let project = if !cwd.is_empty() {
        PathBuf::from(cwd.trim_end_matches('/'))
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| cwd.clone())
    } else {
        PathBuf::from(path)
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
            .rsplit('-')
            .next()
            .unwrap_or("?")
            .to_string()
    };

    con.execute("DELETE FROM msg_fts WHERE session_id=?1", [&session_id])
        .unwrap();
    con.execute("DELETE FROM sessions WHERE session_id=?1", [&session_id])
        .unwrap();
    let n = rows.len();
    if n > 0 {
        let mut ins = con
            .prepare_cached(
                "INSERT INTO msg_fts(text, session_id, role, ts) \
                 VALUES(?1,?2,?3,?4)",
            )
            .unwrap();
        for (text, role, ts) in &rows {
            ins.execute((text, &session_id, role, ts)).unwrap();
        }
        con.execute(
            "INSERT INTO sessions VALUES(?1,?2,?3,?4,?5,?6,?7)",
            (
                &session_id,
                &project,
                &cwd,
                if title.is_empty() { "(no prompt)" } else { &title },
                &first_ts,
                &last_ts,
                n as i64,
            ),
        )
        .unwrap();
    }
    let meta = std::fs::metadata(path).ok();
    let mtime = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let size = meta.map(|m| m.len() as i64).unwrap_or(0);
    con.execute(
        "INSERT OR REPLACE INTO files VALUES(?1,?2,?3,?4)",
        (path, mtime, size, &session_id),
    )
    .unwrap();
    n
}

fn cmd_index() {
    let mut con = open_db();
    let root = home().join(".claude/projects");

    let mut known: HashMap<String, (f64, i64)> = HashMap::new();
    {
        let mut st = con
            .prepare("SELECT path, mtime, size FROM files")
            .unwrap();
        let iter = st
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?)))
            })
            .unwrap();
        for row in iter.flatten() {
            known.insert(row.0, row.1);
        }
    }

    let mut todo: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_string_lossy().into_owned();
        if !path.ends_with(".jsonl") {
            continue;
        }
        seen.insert(path.clone());
        let meta = entry.metadata().ok();
        let mtime = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let size = meta.map(|m| m.len() as i64).unwrap_or(0);
        if known.get(&path) != Some(&(mtime, size)) {
            todo.push(path);
        }
    }

    // purge deleted session files
    for path in known.keys().filter(|p| !seen.contains(*p)) {
        let sid: Option<String> = con
            .query_row(
                "SELECT session_id FROM files WHERE path=?1",
                [path],
                |r| r.get(0),
            )
            .ok();
        if let Some(sid) = sid {
            con.execute("DELETE FROM msg_fts WHERE session_id=?1", [&sid])
                .unwrap();
            con.execute("DELETE FROM sessions WHERE session_id=?1", [&sid])
                .unwrap();
        }
        con.execute("DELETE FROM files WHERE path=?1", [path]).unwrap();
    }

    let total_files = todo.len();
    let mut total_msgs = 0usize;
    let tx = con.transaction().unwrap();
    for (i, path) in todo.iter().enumerate() {
        total_msgs += index_file(&tx, path);
        if (i + 1) % 50 == 0 {
            eprint!("\r  indexing {}/{} sessions…", i + 1, total_files);
            std::io::stderr().flush().ok();
        }
    }
    tx.commit().unwrap();
    if total_files > 0 {
        eprintln!(
            "\rIndexed {total_files} session(s), {total_msgs} messages.      "
        );
    }
}

// ---------------- list ----------------

fn emit(out: &mut impl Write, sid: &str, cwd: &str, ts: &str, project: &str,
        title: &str) {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let proj = truncate_chars(project, 18);
    writeln!(
        out,
        "{sid}\t{cwd}\t{DIM}{age:>4}{RESET}\t{CYAN}{proj:<18}{RESET}\t{t}",
        age = age_str(ts),
        t = truncate_chars(&title, 120),
    )
    .ok();
}

fn cmd_list(raw: &str) {
    let con = open_db();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let Some(matchq) = fts_query(raw) else {
        let mut st = con
            .prepare(
                "SELECT session_id, cwd, last_ts, project, title \
                 FROM sessions ORDER BY last_ts DESC LIMIT ?1",
            )
            .unwrap();
        let rows = st
            .query_map([LIMIT as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .unwrap();
        for (sid, cwd, ts, project, title) in rows.flatten() {
            emit(&mut out, &sid, cwd.as_deref().unwrap_or(""),
                 ts.as_deref().unwrap_or(""), &project, &title);
        }
        return;
    };

    let mut best: HashMap<String, f64> = HashMap::new();
    {
        let mut st = con
            .prepare(
                "SELECT session_id, rank FROM msg_fts \
                 WHERE msg_fts MATCH ?1 ORDER BY rank LIMIT 2000",
            )
            .unwrap();
        let Ok(iter) = st.query_map([&matchq], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        }) else {
            return; // malformed query mid-typing: show nothing
        };
        for (sid, rank) in iter.flatten() {
            best.entry(sid).or_insert(rank);
        }
    }
    if best.is_empty() {
        return;
    }

    let placeholders =
        std::iter::repeat("?").take(best.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT session_id, cwd, last_ts, project, title FROM sessions \
         WHERE session_id IN ({placeholders})"
    );
    let mut st = con.prepare(&sql).unwrap();
    let params: Vec<&String> = best.keys().collect();
    let mut rows: Vec<(String, String, String, String, String)> = st
        .query_map(rusqlite::params_from_iter(params), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .unwrap()
        .flatten()
        .collect();
    rows.sort_by(|a, b| {
        let sa = best[&a.0] + 0.05 * age_days(&a.2);
        let sb = best[&b.0] + 0.05 * age_days(&b.2);
        sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (sid, cwd, ts, project, title) in rows.into_iter().take(LIMIT) {
        emit(&mut out, &sid, &cwd, &ts, &project, &title);
    }
}

// ---------------- preview ----------------

fn cmd_preview(sid: &str, raw: &str) {
    let con = open_db();
    let Ok((project, cwd, title, _first, last_ts, msgs)) = con.query_row(
        "SELECT project, cwd, title, first_ts, last_ts, msgs \
         FROM sessions WHERE session_id=?1",
        [sid],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                r.get::<_, i64>(5)?,
            ))
        },
    ) else {
        return;
    };
    let date = truncate_chars(&last_ts, 16).replace('T', " ");
    println!("{BOLD}{title}{RESET}");
    println!("{DIM}{project} · {cwd} · {msgs} msgs · last {date}{RESET}\n");

    let tag = |role: &str| if role == "user" { "you" } else { "claude" };

    if let Some(matchq) = fts_query(raw) {
        let mut st = con
            .prepare(
                "SELECT role, \
                        snippet(msg_fts, 0, ?1, ?2, '…', 28) \
                 FROM msg_fts WHERE msg_fts MATCH ?3 AND session_id=?4 \
                 ORDER BY rank LIMIT 6",
            )
            .unwrap();
        let hi = format!("{YELLOW}{BOLD}");
        let Ok(iter) = st.query_map((&hi, RESET, &matchq, sid), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) else {
            return;
        };
        for (role, snip) in iter.flatten() {
            let snip = snip.split_whitespace().collect::<Vec<_>>().join(" ");
            println!("{DIM}[{}]{RESET} {snip}\n", tag(&role));
        }
    } else {
        let mut st = con
            .prepare(
                "SELECT role, text FROM msg_fts WHERE session_id=?1 \
                 ORDER BY ts DESC LIMIT 4",
            )
            .unwrap();
        let mut tail: Vec<(String, String)> = st
            .query_map([sid], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .flatten()
            .collect();
        tail.reverse();
        for (role, text) in tail {
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            println!("{DIM}[{}]{RESET} {}\n", tag(&role),
                     truncate_chars(&text, 400));
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("index") => cmd_index(),
        Some("list") => cmd_list(args.get(2).map(String::as_str).unwrap_or("")),
        Some("preview") => {
            if let Some(sid) = args.get(2) {
                cmd_preview(sid, args.get(3).map(String::as_str).unwrap_or(""));
            }
        }
        _ => eprintln!("usage: claude-sessions-core index|list [q]|preview SID [q]"),
    }
}
