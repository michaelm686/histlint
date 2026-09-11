//! Lint rules. Each rule looks at one history entry in isolation and
//! decides whether to emit a finding. Rules are intentionally conservative:
//! a false negative is annoying, a false positive on every third line makes
//! people stop reading the output.

use crate::history::Entry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Danger,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Danger => "danger",
        }
    }
}

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub command: String,
}

pub fn lint(entries: &[Entry]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for entry in entries {
        check_plaintext_credentials(entry, &mut findings);
        check_pipe_to_shell(entry, &mut findings);
        check_destructive_rm(entry, &mut findings);
        check_eval_remote(entry, &mut findings);
    }
    findings
}

fn push(
    findings: &mut Vec<Finding>,
    entry: &Entry,
    rule: &'static str,
    severity: Severity,
    message: impl Into<String>,
) {
    findings.push(Finding {
        line: entry.line,
        rule,
        severity,
        message: message.into(),
        command: entry.command.clone(),
    });
}

// Shell history is stored in plain text with no access controls beyond
// normal file permissions, so anything that looks like a credential typed
// straight into a command line is a finding regardless of the tool involved.
fn check_plaintext_credentials(entry: &Entry, findings: &mut Vec<Finding>) {
    let lower = entry.command.to_lowercase();
    let keyword_hits = [
        "password=",
        "passwd=",
        "secret=",
        "apikey=",
        "api_key=",
        "token=",
    ];
    for kw in keyword_hits {
        if lower.contains(kw) {
            push(
                findings,
                entry,
                "plaintext-credential",
                Severity::Danger,
                format!(
                    "command embeds a credential after `{}`",
                    kw.trim_end_matches('=')
                ),
            );
            return;
        }
    }

    // mysql/psql style: -pSECRET with no space before the value.
    if let Some(pos) = entry.command.find(" -p") {
        let rest = &entry.command[pos + 3..];
        if rest.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false) {
            push(
                findings,
                entry,
                "plaintext-credential",
                Severity::Warning,
                "password appears to be passed directly via -p with no space",
            );
            return;
        }
    }

    // scheme://user:pass@host embedded in a URL.
    if let Some(scheme_end) = entry.command.find("://") {
        let after_scheme = &entry.command[scheme_end + 3..];
        if let Some(at) = after_scheme.find('@') {
            let creds = &after_scheme[..at];
            if creds.contains(':') && !creds.contains('/') && !creds.is_empty() {
                push(
                    findings,
                    entry,
                    "plaintext-credential",
                    Severity::Danger,
                    "URL embeds basic-auth credentials before the @",
                );
            }
        }
    }
}

// Downloading a script and piping it straight into an interpreter means
// running whatever the server hands back that day, unreviewed.
fn check_pipe_to_shell(entry: &Entry, findings: &mut Vec<Finding>) {
    let cmd = &entry.command;
    if !cmd.contains("curl") && !cmd.contains("wget") {
        return;
    }
    for stage in cmd.split('|').skip(1) {
        let first_word = stage.trim().split_whitespace().next().unwrap_or("");
        if matches!(first_word, "sh" | "bash" | "zsh") {
            push(
                findings,
                entry,
                "pipe-to-shell",
                Severity::Danger,
                "downloaded script is piped directly into a shell without inspection",
            );
            return;
        }
    }
}

// rm -rf against a small set of catastrophic targets.
fn check_destructive_rm(entry: &Entry, findings: &mut Vec<Finding>) {
    let cmd = entry.command.trim();
    let looks_like_rm = cmd.split_whitespace().next() == Some("rm")
        || cmd.contains("; rm ")
        || cmd.contains("&& rm ")
        || cmd.contains("| rm ");
    if !looks_like_rm {
        return;
    }
    if !cmd.contains("-r") && !cmd.contains("-R") {
        return;
    }
    let dangerous_targets = ["/", "~", "/*", "$HOME", "*"];
    for word in cmd.split_whitespace() {
        if dangerous_targets.contains(&word) {
            push(
                findings,
                entry,
                "destructive-rm",
                Severity::Danger,
                format!("recursive delete targets `{}`", word),
            );
            return;
        }
    }
}

// eval + a network fetch is the shell equivalent of running unreviewed
// remote code, distinct from the simpler pipe-to-shell case (e.g. `eval
// "$(curl ...)"`).
fn check_eval_remote(entry: &Entry, findings: &mut Vec<Finding>) {
    let cmd = &entry.command;
    if cmd.contains("eval") && (cmd.contains("curl") || cmd.contains("wget")) {
        push(
            findings,
            entry,
            "eval-remote",
            Severity::Danger,
            "eval combined with a network fetch runs unreviewed remote content",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Entry;

    fn entry(command: &str) -> Entry {
        Entry {
            line: 1,
            command: command.to_string(),
        }
    }

    #[test]
    fn flags_password_assignment() {
        let e = entry("mysqldump --password=hunter2 mydb > backup.sql");
        let findings = lint(std::slice::from_ref(&e));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "plaintext-credential");
    }

    #[test]
    fn flags_curl_pipe_bash() {
        let e = entry("curl -sSL https://example.com/install.sh | bash");
        let findings = lint(std::slice::from_ref(&e));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "pipe-to-shell");
    }

    #[test]
    fn flags_rm_rf_root() {
        let e = entry("sudo rm -rf /");
        let findings = lint(std::slice::from_ref(&e));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "destructive-rm");
    }

    #[test]
    fn ignores_ordinary_commands() {
        let e = entry("git commit -m 'fix typo'");
        let findings = lint(std::slice::from_ref(&e));
        assert!(findings.is_empty());
    }
}
