use std::env;
use std::fs;
use std::process::ExitCode;

mod history;
mod report;
mod rules;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut json = false;
    let mut path: Option<String> = None;

    for arg in &args {
        match arg.as_str() {
            "--json" => json = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other if path.is_none() => path = Some(other.to_string()),
            other => {
                eprintln!("histlint: unexpected argument `{}`", other);
                print_usage();
                return ExitCode::FAILURE;
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("histlint: could not read {}: {}", path, e);
            return ExitCode::FAILURE;
        }
    };

    let entries = history::parse(&contents);
    let findings = rules::lint(&entries);

    if json {
        report::print_json(&findings);
    } else {
        report::print_human(&findings);
    }

    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_usage() {
    eprintln!("usage: histlint [--json] <history-file>");
    eprintln!();
    eprintln!("examples:");
    eprintln!("  histlint ~/.bash_history");
    eprintln!("  histlint --json ~/.zsh_history");
}
