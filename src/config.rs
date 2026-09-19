//! Per-project config: a plain text file (default `.histlintrc` in the
//! current directory) that turns individual rules off. Kept deliberately
//! small rather than pulling in a TOML/YAML parser we don't need for one
//! knob per rule.
//!
//! Format, one directive per line:
//!
//!   # comment
//!   disable = pipe-to-shell
//!   disable = eval-remote, destructive-rm
//!
//! Unknown rule names are rejected at load time rather than silently
//! ignored, so a typo in the config doesn't quietly turn off the wrong
//! check (or nothing at all).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub const RULE_NAMES: &[&str] = &[
    "plaintext-credential",
    "pipe-to-shell",
    "eval-remote",
    "destructive-rm",
];

pub const DEFAULT_FILENAME: &str = ".histlintrc";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Config {
    disabled: HashSet<String>,
}

impl Config {
    pub fn is_disabled(&self, rule: &str) -> bool {
        self.disabled.contains(rule)
    }
}

/// Loads `.histlintrc` from the current directory if it exists. Returns an
/// empty config (nothing disabled) if there's no file to load, since having
/// no config is the common case, not an error.
pub fn load_default() -> Result<Config, String> {
    if Path::new(DEFAULT_FILENAME).exists() {
        load(DEFAULT_FILENAME)
    } else {
        Ok(Config::default())
    }
}

pub fn load(path: &str) -> Result<Config, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("could not read {}: {}", path, e))?;
    parse(&contents)
}

fn parse(contents: &str) -> Result<Config, String> {
    let mut disabled = HashSet::new();

    for (idx, raw_line) in contents.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected `key = value`, got `{}`", line_no, line))?;
        let key = key.trim();
        if key != "disable" {
            return Err(format!("line {}: unknown directive `{}`", line_no, key));
        }

        for rule in value.split(',') {
            let rule = rule.trim();
            if rule.is_empty() {
                continue;
            }
            if !RULE_NAMES.contains(&rule) {
                return Err(format!(
                    "line {}: unknown rule `{}` (known rules: {})",
                    line_no,
                    rule,
                    RULE_NAMES.join(", ")
                ));
            }
            disabled.insert(rule.to_string());
        }
    }

    Ok(Config { disabled })
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(pos) => &line[..pos],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_disables_nothing() {
        let cfg = parse("").unwrap();
        assert!(!cfg.is_disabled("pipe-to-shell"));
    }

    #[test]
    fn disables_a_single_rule() {
        let cfg = parse("disable = pipe-to-shell\n").unwrap();
        assert!(cfg.is_disabled("pipe-to-shell"));
        assert!(!cfg.is_disabled("eval-remote"));
    }

    #[test]
    fn disables_a_comma_separated_list() {
        let cfg = parse("disable = eval-remote, destructive-rm\n").unwrap();
        assert!(cfg.is_disabled("eval-remote"));
        assert!(cfg.is_disabled("destructive-rm"));
        assert!(!cfg.is_disabled("plaintext-credential"));
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let input = "# turn off the noisy one\n\ndisable = pipe-to-shell\n";
        let cfg = parse(input).unwrap();
        assert!(cfg.is_disabled("pipe-to-shell"));
    }

    #[test]
    fn accumulates_across_multiple_disable_lines() {
        let input = "disable = pipe-to-shell\ndisable = eval-remote\n";
        let cfg = parse(input).unwrap();
        assert!(cfg.is_disabled("pipe-to-shell"));
        assert!(cfg.is_disabled("eval-remote"));
    }

    #[test]
    fn rejects_unknown_rule_names() {
        let err = parse("disable = not-a-real-rule\n").unwrap_err();
        assert!(err.contains("not-a-real-rule"));
    }

    #[test]
    fn rejects_unknown_directives() {
        let err = parse("enable = pipe-to-shell\n").unwrap_err();
        assert!(err.contains("unknown directive"));
    }

    #[test]
    fn rejects_lines_without_equals() {
        let err = parse("disable pipe-to-shell\n").unwrap_err();
        assert!(err.contains("expected `key = value`"));
    }
}
