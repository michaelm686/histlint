//! Two output modes over the same findings: human-readable text for a
//! terminal, and a JSON array for scripts (CI, editor integrations, etc).
//! Written by hand since the project has no dependencies to serialize with.

use crate::rules::Finding;

pub fn print_human(findings: &[Finding]) {
    if findings.is_empty() {
        println!("no findings");
        return;
    }

    for f in findings {
        println!(
            "{}: [{}] {}: {}",
            f.line,
            f.severity.as_str(),
            f.rule,
            f.message
        );
        println!("    {}", f.command);
    }

    println!();
    println!("{} finding(s)", findings.len());
}

pub fn print_json(findings: &[Finding]) {
    let mut out = String::from("[");
    for (i, f) in findings.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"line\":");
        out.push_str(&f.line.to_string());
        out.push_str(",\"rule\":\"");
        out.push_str(f.rule);
        out.push_str("\",\"severity\":\"");
        out.push_str(f.severity.as_str());
        out.push_str("\",\"message\":");
        out.push_str(&json_string(&f.message));
        out.push_str(",\"command\":");
        out.push_str(&json_string(&f.command));
        out.push('}');
    }
    out.push(']');
    println!("{}", out);
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Severity;

    #[test]
    fn escapes_quotes_and_newlines() {
        assert_eq!(json_string("say \"hi\"\n"), "\"say \\\"hi\\\"\\n\"");
    }

    #[test]
    fn json_output_is_a_valid_looking_array() {
        let findings = vec![Finding {
            line: 3,
            rule: "destructive-rm",
            severity: Severity::Danger,
            message: "recursive delete targets `/`".to_string(),
            command: "rm -rf /".to_string(),
        }];
        // print_json writes to stdout; here we just exercise json_string,
        // which is the part worth unit testing in isolation.
        let escaped = json_string(&findings[0].command);
        assert_eq!(escaped, "\"rm -rf /\"");
    }
}
