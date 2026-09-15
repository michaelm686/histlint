//! Parsing for the shell history formats we care about: plain bash history
//! (one command per line) and zsh extended history (`: <epoch>:<duration>;command`,
//! with backslash-continued lines for multi-line commands).

pub struct Entry {
    /// 1-based line number in the source file where this entry starts.
    pub line: usize,
    pub command: String,
}

pub fn parse(input: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;

        if let Some(rest) = raw_line.strip_prefix(": ") {
            // zsh extended history: ": <epoch>:<duration>;<command>"
            if let Some(semicolon) = rest.find(';') {
                if let Some(entry) = current.take() {
                    entries.push(entry);
                }
                current = Some(Entry {
                    line: line_no,
                    command: rest[semicolon + 1..].to_string(),
                });
                continue;
            }
        }

        if let Some(entry) = current.as_mut() {
            if entry.command.ends_with('\\') {
                // continuation of a multi-line command
                entry.command.pop();
                entry.command.push('\n');
                entry.command.push_str(raw_line);
                continue;
            }
            entries.push(current.take().unwrap());
        }

        if raw_line.trim().is_empty() {
            continue;
        }

        current = Some(Entry {
            line: line_no,
            command: strip_history_number(raw_line).to_string(),
        });
    }

    if let Some(entry) = current.take() {
        entries.push(entry);
    }

    entries
}

// The `history` builtin prints each entry as "  501  the command", so a
// file piped straight from `history` (rather than read from a history file
// on disk) needs that counter stripped before the rest of the rules see it.
// Real history files don't produce this shape, so applying it unconditionally
// is safe: it only ever matches a leading run of digits followed by a space.
fn strip_history_number(line: &str) -> &str {
    let trimmed = line.trim_start();
    let digit_end = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    if digit_end == 0 {
        return line;
    }
    match trimmed[digit_end..].strip_prefix(' ') {
        Some(rest) => rest.trim_start_matches(' '),
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_history() {
        let input = "ls -la\ncd /tmp\n";
        let entries = parse(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].line, 1);
        assert_eq!(entries[0].command, "ls -la");
        assert_eq!(entries[1].line, 2);
    }

    #[test]
    fn parses_zsh_extended_history() {
        let input = ": 1700000000:0;git status\n: 1700000001:2;cargo build\n";
        let entries = parse(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "git status");
        assert_eq!(entries[1].command, "cargo build");
    }

    #[test]
    fn joins_backslash_continuations() {
        let input = "echo one \\\ntwo\n";
        let entries = parse(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "echo one \ntwo");
    }

    #[test]
    fn strips_history_builtin_numbering() {
        let input = "  501  git status\n  502  cargo build\n";
        let entries = parse(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "git status");
        assert_eq!(entries[1].command, "cargo build");
    }

    #[test]
    fn leaves_numberless_lines_alone() {
        let input = "2to3 script.py\n";
        let entries = parse(input);
        assert_eq!(entries[0].command, "2to3 script.py");
    }
}
