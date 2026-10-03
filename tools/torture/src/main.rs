//! CLI entry point for the T00/H00 corpus-lock verifier.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use cc_silicon_torture::verify::{verify_lock_text, VerifyOptions, DEFAULT_MAX_TRAVERSAL_ENTRIES};
use cc_silicon_torture::TOOL_VERSION;

const USAGE: &str = "\
usage: cc-silicon-torture <LOCK> <CORPUS_ROOT> [--check-unlisted] [--max-unlisted-entries <N>]

Verifies an explicitly supplied local corpus lock against a local asset tree.
This tool never downloads corpus assets and never invokes a C compiler.

Arguments:
  <LOCK>                       Path to a cc-silicon.torture/lock/v1 lock file
  <CORPUS_ROOT>                Directory holding the local (fetched) corpus assets
  --check-unlisted             Also report tree files that the lock does not list
  --max-unlisted-entries <N>   Bound the --check-unlisted traversal (0 disables)

Exit codes:
  0  frozen lock verified (no errors)
  1  verification failed (errors present)
  2  usage or operational error (unreadable lock file)
  3  lock is structurally valid but draft/unfrozen; H00 cannot be claimed
";

fn main() -> ExitCode {
    ExitCode::from(run(std::env::args_os().skip(1).collect()))
}

fn run(args: Vec<OsString>) -> u8 {
    let mut check_unlisted = false;
    let mut max_traversal_entries = DEFAULT_MAX_TRAVERSAL_ENTRIES;
    let mut positionals: Vec<PathBuf> = Vec::new();

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        // Flags must be valid UTF-8; anything else is treated as a path so that
        // non-UTF-8 lock/root arguments never panic.
        match arg.to_str() {
            Some("-h") | Some("--help") => {
                print!("{USAGE}");
                return 0;
            }
            Some("-V") | Some("--version") => {
                println!("cc-silicon-torture {TOOL_VERSION}");
                return 0;
            }
            Some("--check-unlisted") => check_unlisted = true,
            Some("--max-unlisted-entries") => {
                let Some(value) = iter.next() else {
                    eprintln!("error: --max-unlisted-entries requires a value\n\n{USAGE}");
                    return 2;
                };
                match value.to_str().and_then(|text| text.parse::<usize>().ok()) {
                    Some(parsed) => max_traversal_entries = parsed,
                    None => {
                        eprintln!(
                            "error: --max-unlisted-entries expects a non-negative integer\n\n{USAGE}"
                        );
                        return 2;
                    }
                }
            }
            Some(other) if other.starts_with('-') && other != "-" => {
                eprintln!("error: unknown option '{other}'\n\n{USAGE}");
                return 2;
            }
            _ => positionals.push(PathBuf::from(arg)),
        }
    }

    if positionals.len() != 2 {
        eprintln!("error: expected <LOCK> and <CORPUS_ROOT>\n\n{USAGE}");
        return 2;
    }

    let lock_path = &positionals[0];
    let root = &positionals[1];

    let text = match std::fs::read_to_string(lock_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "error: cannot read lock file {}: {error}",
                lock_path.display()
            );
            return 2;
        }
    };

    let options = VerifyOptions {
        check_unlisted,
        max_traversal_entries,
    };
    let report = verify_lock_text(&text, &lock_path.display().to_string(), root, &options);
    print!("{}", report.render());
    report.exit_code() as u8
}
