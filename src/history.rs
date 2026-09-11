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
            command: raw_line.to_string(),
        });
    }

    if let Some(entry) = current.take() {
        entries.push(entry);
    }

    entries
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
}
