use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

mod config;
mod history;
mod report;
mod rules;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut json = false;
    let mut path: Option<String> = None;
    let mut config_path: Option<String> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--config" => match iter.next() {
                Some(value) => config_path = Some(value.to_string()),
                None => {
                    eprintln!("histlint: --config requires a path argument");
                    print_usage();
                    return ExitCode::FAILURE;
                }
            },
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

    let config = match config_path.as_deref() {
        Some(p) => config::load(p),
        None => config::load_default(),
    };
    let config = match config {
        Ok(c) => c,
        Err(e) => {
            eprintln!("histlint: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let contents = match path.as_deref() {
        None | Some("-") => {
            let mut buf = String::new();
            match io::stdin().read_to_string(&mut buf) {
                Ok(_) => buf,
                Err(e) => {
                    eprintln!("histlint: could not read stdin: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        }
        Some(p) => match fs::read_to_string(p) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("histlint: could not read {}: {}", p, e);
                return ExitCode::FAILURE;
            }
        },
    };

    let entries = history::parse(&contents);
    let findings: Vec<_> = rules::lint(&entries)
        .into_iter()
        .filter(|f| !config.is_disabled(f.rule))
        .collect();

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
    eprintln!("usage: histlint [--json] [--config <path>] [history-file | -]");
    eprintln!();
    eprintln!("with no file, or with `-`, reads from stdin");
    eprintln!(
        "without --config, looks for `{}` in the current directory",
        config::DEFAULT_FILENAME
    );
    eprintln!();
    eprintln!("examples:");
    eprintln!("  histlint ~/.bash_history");
    eprintln!("  histlint --json ~/.zsh_history");
    eprintln!("  histlint --config ci/histlintrc ~/.bash_history");
    eprintln!("  history | histlint");
}
