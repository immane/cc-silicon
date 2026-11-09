//! CLI for the H05 per-instance isolated runner.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use cc_silicon_runner::{run, CommandSpec, ResourceLimits, RunSpec};

const USAGE: &str = "\
usage: cc-silicon-runner --workdir <dir> [options] -- <program> [args...]

Options:
  --workdir <dir>          per-instance working directory (required)
  --timeout-ms <n>         wall-clock timeout (default 10000)
  --stdout-limit <n>       stdout byte cap, 0 = unlimited (default 1048576)
  --stderr-limit <n>       stderr byte cap, 0 = unlimited (default 1048576)
  --cancel-file <path>     terminate when this marker appears
  --rlimit-as <bytes>      address-space bound (Linux prlimit)
  --rlimit-cpu <seconds>   CPU-time bound (Linux prlimit)
  --rlimit-nofile <n>      open-file bound (Linux prlimit)
  --rlimit-nproc <n>       process/thread bound (Linux prlimit)
  --help                   show this help
  --version                show the tool version

The runner clears the environment to PATH, LC_ALL=C, LANG=C, HOME=<workdir>,
TMPDIR=<workdir>/tmp. It never invokes a shell unless the command itself is a
shell. Linux process-group isolation uses setsid+kill when available.
";

fn main() -> ExitCode {
    ExitCode::from(run_cli(std::env::args_os().skip(1).collect()))
}

fn run_cli(args: Vec<OsString>) -> u8 {
    let mut workdir: Option<PathBuf> = None;
    let mut timeout_ms: u64 = 10_000;
    let mut stdout_limit: usize = 1_048_576;
    let mut stderr_limit: usize = 1_048_576;
    let mut cancel_file: Option<PathBuf> = None;
    let mut limits = ResourceLimits::default();
    let mut command: Option<CommandSpec> = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if command.is_some() {
            let Some(spec) = command.as_mut() else {
                return 2;
            };
            spec.args.push(arg);
            continue;
        }
        match arg.to_str() {
            Some("-h") | Some("--help") => {
                print!("{USAGE}");
                return 0;
            }
            Some("-V") | Some("--version") => {
                println!("cc-silicon-runner {}", env!("CARGO_PKG_VERSION"));
                return 0;
            }
            Some("--") => {
                let Some(program) = iter.next() else {
                    eprintln!("error: `--` must be followed by a program\n\n{USAGE}");
                    return 2;
                };
                command = Some(CommandSpec {
                    program,
                    args: Vec::new(),
                });
            }
            Some("--workdir") => {
                let Some(value) = iter.next() else {
                    eprintln!("error: --workdir requires a directory\n\n{USAGE}");
                    return 2;
                };
                workdir = Some(PathBuf::from(value));
            }
            Some("--timeout-ms") => match next_u64(&mut iter, "--timeout-ms") {
                Ok(value) => timeout_ms = value,
                Err(code) => return code,
            },
            Some("--stdout-limit") => match next_usize(&mut iter, "--stdout-limit") {
                Ok(value) => stdout_limit = value,
                Err(code) => return code,
            },
            Some("--stderr-limit") => match next_usize(&mut iter, "--stderr-limit") {
                Ok(value) => stderr_limit = value,
                Err(code) => return code,
            },
            Some("--cancel-file") => {
                let Some(value) = iter.next() else {
                    eprintln!("error: --cancel-file requires a path\n\n{USAGE}");
                    return 2;
                };
                cancel_file = Some(PathBuf::from(value));
            }
            Some("--rlimit-as") => match next_u64(&mut iter, "--rlimit-as") {
                Ok(value) => limits.address_space = Some(value),
                Err(code) => return code,
            },
            Some("--rlimit-cpu") => match next_u64(&mut iter, "--rlimit-cpu") {
                Ok(value) => limits.cpu_seconds = Some(value),
                Err(code) => return code,
            },
            Some("--rlimit-nofile") => match next_u64(&mut iter, "--rlimit-nofile") {
                Ok(value) => limits.nofile = Some(value),
                Err(code) => return code,
            },
            Some("--rlimit-nproc") => match next_u64(&mut iter, "--rlimit-nproc") {
                Ok(value) => limits.nproc = Some(value),
                Err(code) => return code,
            },
            Some(other) if other.starts_with('-') => {
                eprintln!("error: unknown option `{other}`\n\n{USAGE}");
                return 2;
            }
            _ => {
                command = Some(CommandSpec {
                    program: arg,
                    args: Vec::new(),
                });
            }
        }
    }
    let Some(command) = command else {
        eprintln!("error: missing program\n\n{USAGE}");
        return 2;
    };
    let Some(workdir) = workdir else {
        eprintln!("error: --workdir is required\n\n{USAGE}");
        return 2;
    };
    if timeout_ms == 0 {
        eprintln!("error: --timeout-ms must be positive\n\n{USAGE}");
        return 2;
    }
    let spec = RunSpec {
        workdir,
        timeout: Duration::from_millis(timeout_ms),
        stdout_limit,
        stderr_limit,
        cancel_file,
        limits,
        command,
    };
    match run(&spec) {
        Ok(report) => {
            // Existing keys stay in their original order; H01 fields follow.
            println!("outcome={}", report.outcome.label());
            println!("exit_code={}", report.outcome.exit_code());
            println!("elapsed_ms={}", report.elapsed_ms);
            println!("workdir={}", report_path(&report.workdir));
            println!("stdout={}", report_path(&report.workdir.join("stdout.log")));
            println!("stderr={}", report_path(&report.workdir.join("stderr.log")));
            println!("stdout_truncated={}", report.stdout_truncated);
            println!("stderr_truncated={}", report.stderr_truncated);
            println!("child_exit={}", report_optional_i32(report.child_exit));
            println!("signal={}", report_optional_i32(report.signal));
            println!("stdout_bytes={}", report.stdout_bytes);
            println!("stderr_bytes={}", report.stderr_bytes);
            println!("stdout_exceeded={}", report.stdout_exceeded);
            println!("stderr_exceeded={}", report.stderr_exceeded);
            println!("process_group={}", report.process_group);
            println!("prlimit_applied={}", report.prlimit_applied);
            report.outcome.exit_code()
        }
        Err(error) => {
            eprintln!(
                "cc-silicon-runner: {}",
                escape_report_value(&error.to_string())
            );
            2
        }
    }
}

/// Keep each report line self-contained; escapes are reversible for `\`, CR,
/// LF, tab, and other control characters inherited from a path.
fn escape_report_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => {
                let _ = write!(escaped, "\\u{{{:x}}}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped
}

fn report_path(path: &Path) -> String {
    escape_report_value(path.to_string_lossy().as_ref())
}

fn report_optional_i32(value: Option<i32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "none".to_string(),
    }
}

fn next_u64(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<u64, u8> {
    let Some(value) = iter.next() else {
        eprintln!("error: {flag} requires a value");
        return Err(2);
    };
    value
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .ok_or_else(|| {
            eprintln!("error: {flag} expects an unsigned integer");
            2
        })
}

fn next_usize(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<usize, u8> {
    let Some(value) = iter.next() else {
        eprintln!("error: {flag} requires a value");
        return Err(2);
    };
    value
        .to_str()
        .and_then(|text| text.parse::<usize>().ok())
        .ok_or_else(|| {
            eprintln!("error: {flag} expects an unsigned integer");
            2
        })
}
