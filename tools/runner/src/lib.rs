//! H05 per-instance isolated process runner.
//!
//! The runner is a host tool, not a compiler component. It creates/uses one
//! instance directory, clears the environment to a deterministic minimum,
//! enforces a wall-clock timeout and stdout/stderr caps, polls an optional
//! cancellation file, optionally applies Linux `prlimit` resource bounds, and
//! classifies completion/timeout/cancellation/output-limit/signal outcomes.
//!
//! It deliberately uses no `unsafe` code. On Linux, process-group isolation
//! and resource bounds are provided by the external `setsid`/`prlimit` tools
//! when available; absent `prlimit` with requested limits is a setup error,
//! never a silently unbounded run.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Output stream name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamName {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

impl StreamName {
    /// Stable label.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

/// Optional resource bounds applied through Linux `prlimit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Address-space bytes (`RLIMIT_AS`).
    pub address_space: Option<u64>,
    /// CPU seconds (`RLIMIT_CPU`).
    pub cpu_seconds: Option<u64>,
    /// Open files (`RLIMIT_NOFILE`).
    pub nofile: Option<u64>,
    /// Processes/threads (`RLIMIT_NPROC`).
    pub nproc: Option<u64>,
}

impl ResourceLimits {
    /// Whether any bound is requested.
    pub const fn any(self) -> bool {
        self.address_space.is_some()
            || self.cpu_seconds.is_some()
            || self.nofile.is_some()
            || self.nproc.is_some()
    }
}

/// One command to execute.
#[derive(Clone, Debug)]
pub struct CommandSpec {
    /// Executable path or name.
    pub program: OsString,
    /// Arguments (no shell interpretation).
    pub args: Vec<OsString>,
}

/// Complete runner specification.
#[derive(Clone, Debug)]
pub struct RunSpec {
    /// Per-instance directory (created when absent).
    pub workdir: PathBuf,
    /// Wall-clock timeout.
    pub timeout: Duration,
    /// Stdout byte cap (`0` = unlimited).
    pub stdout_limit: usize,
    /// Stderr byte cap (`0` = unlimited).
    pub stderr_limit: usize,
    /// Optional cancellation marker path.
    pub cancel_file: Option<PathBuf>,
    /// Optional Linux resource bounds.
    pub limits: ResourceLimits,
    /// Command to run.
    pub command: CommandSpec,
}

/// Why the runner terminated the child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// Normal/waitable completion.
    Completed,
    /// Wall-clock timeout.
    Timeout,
    /// Cancellation file appeared.
    Cancelled,
    /// Output cap exceeded on the named stream.
    OutputLimit(StreamName),
}

/// Classified outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// The process exited with a code.
    Completed {
        /// Exit code.
        code: i32,
    },
    /// The process was terminated by a signal.
    Signaled {
        /// Signal number.
        signal: i32,
    },
    /// Wall-clock timeout.
    Timeout,
    /// Cancellation file observed.
    Cancelled,
    /// Output cap exceeded.
    OutputLimit {
        /// Stream that exceeded.
        stream: StreamName,
    },
}

impl RunOutcome {
    /// Stable label.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Completed { .. } => "completed",
            Self::Signaled { .. } => "signaled",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::OutputLimit { .. } => "output-limit",
        }
    }

    /// Runner exit code.
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Completed { code: 0 } => 0,
            Self::Completed { .. } => 1,
            Self::Signaled { .. } => 6,
            Self::Timeout => 3,
            Self::Cancelled => 4,
            Self::OutputLimit { .. } => 5,
        }
    }
}

/// Result of one run.
#[derive(Clone, Debug)]
pub struct RunReport {
    /// Classified outcome.
    pub outcome: RunOutcome,
    /// Captured stdout bytes (capped).
    pub stdout: Vec<u8>,
    /// Captured stderr bytes (capped).
    pub stderr: Vec<u8>,
    /// Whether stdout was truncated.
    pub stdout_truncated: bool,
    /// Whether stderr was truncated.
    pub stderr_truncated: bool,
    /// Elapsed wall-clock milliseconds.
    pub elapsed_ms: u128,
    /// Instance directory.
    pub workdir: PathBuf,
}

/// Setup/runner failure (not a child outcome).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupError {
    /// Workdir could not be prepared.
    Workdir(String),
    /// A required external tool is missing.
    MissingTool(&'static str),
    /// Spawn failed.
    Spawn(String),
    /// Wait failed.
    Wait(String),
    /// Log persistence failed.
    Log(String),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workdir(message) => write!(f, "workdir error: {message}"),
            Self::MissingTool(tool) => write!(f, "missing required host tool: {tool}"),
            Self::Spawn(message) => write!(f, "spawn failed: {message}"),
            Self::Wait(message) => write!(f, "wait failed: {message}"),
            Self::Log(message) => write!(f, "log write failed: {message}"),
        }
    }
}

impl std::error::Error for SetupError {}

/// Execute one command under the H05 supervisor.
pub fn run(spec: &RunSpec) -> Result<RunReport, SetupError> {
    let workdir = prepare_workdir(&spec.workdir)?;
    let tmp = workdir.join("tmp");
    std::fs::create_dir_all(&tmp).map_err(|error| {
        SetupError::Workdir(format!("cannot create {}: {error}", tmp.display()))
    })?;

    if Path::new(&spec.command.program).is_absolute() && !Path::new(&spec.command.program).exists()
    {
        return Err(SetupError::Spawn(format!(
            "{}: no such executable",
            spec.command.program.to_string_lossy()
        )));
    }
    let use_setsid = tool_available("setsid");
    let use_prlimit = spec.limits.any();
    if use_prlimit && !tool_available("prlimit") {
        return Err(SetupError::MissingTool("prlimit"));
    }

    let (mut command, group_kill) = build_command(spec, &workdir, use_setsid, use_prlimit);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&workdir);
    let mut child = command.spawn().map_err(|error| {
        SetupError::Spawn(format!(
            "{}: {error}",
            spec.command.program.to_string_lossy()
        ))
    })?;
    let pid = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SetupError::Spawn("stdout pipe missing".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SetupError::Spawn("stderr pipe missing".to_string()))?;

    let exceeded = Arc::new(AtomicU8::new(0));
    let stdout_buf = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf = Arc::new(Mutex::new(Vec::new()));
    let stdout_thread = spawn_reader(
        stdout,
        spec.stdout_limit,
        StreamName::Stdout,
        Arc::clone(&stdout_buf),
        Arc::clone(&exceeded),
    );
    let stderr_thread = spawn_reader(
        stderr,
        spec.stderr_limit,
        StreamName::Stderr,
        Arc::clone(&stderr_buf),
        Arc::clone(&exceeded),
    );

    let started = Instant::now();
    let (status, stop) = wait_or_stop(&mut child, spec, &exceeded, pid, group_kill)?;
    let elapsed_ms = started.elapsed().as_millis();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let stdout = stdout_buf
        .lock()
        .map_err(|_| SetupError::Wait("stdout buffer poisoned".to_string()))?
        .clone();
    let stderr = stderr_buf
        .lock()
        .map_err(|_| SetupError::Wait("stderr buffer poisoned".to_string()))?
        .clone();
    std::fs::write(workdir.join("stdout.log"), &stdout)
        .map_err(|error| SetupError::Log(format!("stdout.log: {error}")))?;
    std::fs::write(workdir.join("stderr.log"), &stderr)
        .map_err(|error| SetupError::Log(format!("stderr.log: {error}")))?;

    let stdout_truncated = spec.stdout_limit != 0 && stdout.len() >= spec.stdout_limit;
    let stderr_truncated = spec.stderr_limit != 0 && stderr.len() >= spec.stderr_limit;
    let outcome = classify(stop, status);
    Ok(RunReport {
        outcome,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        elapsed_ms,
        workdir,
    })
}

fn prepare_workdir(path: &Path) -> Result<PathBuf, SetupError> {
    if path.exists() && !path.is_dir() {
        return Err(SetupError::Workdir(format!(
            "{} exists and is not a directory",
            path.display()
        )));
    }
    std::fs::create_dir_all(path).map_err(|error| {
        SetupError::Workdir(format!("cannot create {}: {error}", path.display()))
    })?;
    path.canonicalize()
        .map_err(|error| SetupError::Workdir(format!("cannot resolve {}: {error}", path.display())))
}

fn build_command(
    spec: &RunSpec,
    workdir: &Path,
    use_setsid: bool,
    use_prlimit: bool,
) -> (Command, bool) {
    let mut args: Vec<OsString> = Vec::new();
    if use_prlimit {
        args.push("prlimit".into());
        if let Some(value) = spec.limits.address_space {
            args.push(format!("--as={value}").into());
        }
        if let Some(value) = spec.limits.cpu_seconds {
            args.push(format!("--cpu={value}").into());
        }
        if let Some(value) = spec.limits.nofile {
            args.push(format!("--nofile={value}").into());
        }
        if let Some(value) = spec.limits.nproc {
            args.push(format!("--nproc={value}").into());
        }
        args.push("--".into());
    }
    args.push(spec.command.program.clone());
    args.extend(spec.command.args.iter().cloned());

    let mut command = if use_setsid {
        let mut command = Command::new("setsid");
        command.arg("--");
        command.args(&args);
        command
    } else {
        let mut command = Command::new(&spec.command.program);
        command.args(&spec.command.args);
        command
    };
    command.env_clear();
    command.env(
        "PATH",
        std::env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin")),
    );
    command.env("LC_ALL", "C");
    command.env("LANG", "C");
    command.env("HOME", workdir);
    command.env("TMPDIR", workdir.join("tmp"));
    (command, use_setsid)
}

fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
    stream: StreamName,
    buffer: Arc<Mutex<Vec<u8>>>,
    exceeded: Arc<AtomicU8>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    let Ok(mut buffer) = buffer.lock() else {
                        break;
                    };
                    if limit == 0 {
                        buffer.extend_from_slice(&chunk[..read]);
                    } else {
                        let room = limit.saturating_sub(buffer.len());
                        let take = room.min(read);
                        buffer.extend_from_slice(&chunk[..take]);
                        if take < read {
                            let code = match stream {
                                StreamName::Stdout => 1,
                                StreamName::Stderr => 2,
                            };
                            exceeded.store(code, Ordering::SeqCst);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    })
}

fn wait_or_stop(
    child: &mut Child,
    spec: &RunSpec,
    exceeded: &AtomicU8,
    pid: u32,
    group_kill: bool,
) -> Result<(Option<ExitStatus>, StopReason), SetupError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| SetupError::Wait(format!("try_wait: {error}")))?
        {
            return Ok((Some(status), StopReason::Completed));
        }
        if let Some(path) = &spec.cancel_file {
            if path.exists() {
                terminate(child, pid, group_kill);
                let _ = child.wait();
                return Ok((None, StopReason::Cancelled));
            }
        }
        if started.elapsed() >= spec.timeout {
            terminate(child, pid, group_kill);
            let _ = child.wait();
            return Ok((None, StopReason::Timeout));
        }
        let code = exceeded.load(Ordering::SeqCst);
        if code != 0 {
            terminate(child, pid, group_kill);
            let _ = child.wait();
            let stream = if code == 1 {
                StreamName::Stdout
            } else {
                StreamName::Stderr
            };
            return Ok((None, StopReason::OutputLimit(stream)));
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(unix)]
fn terminate(child: &mut Child, pid: u32, group_kill: bool) {
    if group_kill {
        let target = format!("-{pid}");
        let _ = Command::new("/bin/kill")
            .args(["-TERM", "--", &target])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        thread::sleep(Duration::from_millis(30));
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &target])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

#[cfg(not(unix))]
fn terminate(child: &mut Child, _pid: u32, _group_kill: bool) {
    let _ = child.kill();
}

fn classify(stop: StopReason, status: Option<ExitStatus>) -> RunOutcome {
    match stop {
        StopReason::Timeout => RunOutcome::Timeout,
        StopReason::Cancelled => RunOutcome::Cancelled,
        StopReason::OutputLimit(stream) => RunOutcome::OutputLimit { stream },
        StopReason::Completed => classify_status(status),
    }
}

#[cfg(unix)]
fn classify_status(status: Option<ExitStatus>) -> RunOutcome {
    use std::os::unix::process::ExitStatusExt;
    let Some(status) = status else {
        return RunOutcome::Signaled { signal: 0 };
    };
    if let Some(signal) = status.signal() {
        RunOutcome::Signaled { signal }
    } else if let Some(code) = status.code() {
        RunOutcome::Completed { code }
    } else {
        RunOutcome::Signaled { signal: 0 }
    }
}

#[cfg(not(unix))]
fn classify_status(status: Option<ExitStatus>) -> RunOutcome {
    match status.and_then(|status| status.code()) {
        Some(code) => RunOutcome::Completed { code },
        None => RunOutcome::Signaled { signal: 0 },
    }
}
