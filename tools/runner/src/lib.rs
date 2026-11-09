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
//!
//! Cleanup is bounded: a stopped child is reaped and reader threads are awaited
//! against a fixed deadline. A descendant that escapes the supervised process
//! group can still keep a pipe open; the runner then returns without its
//! captured tail rather than blocking forever. `process_group` and
//! `prlimit_applied` in [`RunReport`] disclose which containment mechanisms
//! were actually selected.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const STDOUT_EXCEEDED: u8 = 1 << 0;
const STDERR_EXCEEDED: u8 = 1 << 1;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const TERM_GRACE: Duration = Duration::from_millis(30);
const READER_DRAIN_AFTER_EXIT: Duration = Duration::from_millis(250);
const CLEANUP_TIMEOUT: Duration = Duration::from_millis(1_000);

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
    /// Whether stdout was truncated because the cap was exceeded.
    pub stdout_truncated: bool,
    /// Whether stderr was truncated because the cap was exceeded.
    pub stderr_truncated: bool,
    /// Elapsed wall-clock milliseconds, including cleanup and log persistence.
    pub elapsed_ms: u128,
    /// Instance directory.
    pub workdir: PathBuf,
    /// Waitable child exit code, when the child exited normally.
    pub child_exit: Option<i32>,
    /// Terminating signal, when the child was signaled.
    pub signal: Option<i32>,
    /// Total stdout bytes read, including bytes discarded beyond the cap.
    pub stdout_bytes: usize,
    /// Total stderr bytes read, including bytes discarded beyond the cap.
    pub stderr_bytes: usize,
    /// Whether stdout exceeded its cap (distinct from merely reaching it).
    pub stdout_exceeded: bool,
    /// Whether stderr exceeded its cap (distinct from merely reaching it).
    pub stderr_exceeded: bool,
    /// Whether the command was placed in its own process group via `setsid`.
    pub process_group: bool,
    /// Whether requested resource limits were wrapped with `prlimit`.
    pub prlimit_applied: bool,
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

#[derive(Default)]
struct Capture {
    stdout: Mutex<Vec<u8>>,
    stderr: Mutex<Vec<u8>>,
    stdout_bytes: AtomicUsize,
    stderr_bytes: AtomicUsize,
    exceeded: AtomicU8,
    closed: AtomicBool,
}

impl Capture {
    fn buffer(&self, stream: StreamName) -> &Mutex<Vec<u8>> {
        match stream {
            StreamName::Stdout => &self.stdout,
            StreamName::Stderr => &self.stderr,
        }
    }

    fn bytes(&self, stream: StreamName) -> &AtomicUsize {
        match stream {
            StreamName::Stdout => &self.stdout_bytes,
            StreamName::Stderr => &self.stderr_bytes,
        }
    }

    fn exceeded_mask(&self) -> u8 {
        self.exceeded.load(Ordering::SeqCst)
    }

    fn mark_exceeded(&self, stream: StreamName) {
        let bit = match stream {
            StreamName::Stdout => STDOUT_EXCEEDED,
            StreamName::Stderr => STDERR_EXCEEDED,
        };
        self.exceeded.fetch_or(bit, Ordering::SeqCst);
    }

    fn snapshot(&self, stream: StreamName) -> Vec<u8> {
        lock_recover(self.buffer(stream)).clone()
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Execute one command under the H05 supervisor.
pub fn run(spec: &RunSpec) -> Result<RunReport, SetupError> {
    let started = Instant::now();
    let workdir = prepare_workdir(&spec.workdir)?;
    let tmp = workdir.join("tmp");
    reject_workdir_symlink(&tmp, "tmp")?;
    std::fs::create_dir_all(&tmp).map_err(|error| {
        SetupError::Workdir(format!("cannot create {}: {error}", tmp.display()))
    })?;

    // Validate log targets before spawning so a symlink cannot redirect the
    // child's side effects through a pre-existing runner log path.
    let stdout_log = workdir.join("stdout.log");
    let stderr_log = workdir.join("stderr.log");
    reject_log_target(&stdout_log)?;
    reject_log_target(&stderr_log)?;

    if Path::new(&spec.command.program).is_absolute() && !Path::new(&spec.command.program).exists()
    {
        return Err(SetupError::Spawn(format!(
            "{}: no such executable",
            spec.command.program.to_string_lossy()
        )));
    }
    let process_group = tool_available("setsid");
    let prlimit_requested = spec.limits.any();
    if prlimit_requested && !tool_available("prlimit") {
        return Err(SetupError::MissingTool("prlimit"));
    }

    let (mut command, process_group, prlimit_applied) =
        build_command(spec, &workdir, process_group, prlimit_requested);
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
    let stdout = match child.stdout.take() {
        Some(pipe) => pipe,
        None => {
            abort_spawned(&mut child, pid, process_group);
            return Err(SetupError::Spawn("stdout pipe missing".to_string()));
        }
    };
    let stderr = match child.stderr.take() {
        Some(pipe) => pipe,
        None => {
            abort_spawned(&mut child, pid, process_group);
            return Err(SetupError::Spawn("stderr pipe missing".to_string()));
        }
    };

    let capture = Arc::new(Capture::default());
    let (stdout_thread, stdout_done) = spawn_reader(
        stdout,
        spec.stdout_limit,
        StreamName::Stdout,
        Arc::clone(&capture),
    );
    let (stderr_thread, stderr_done) = spawn_reader(
        stderr,
        spec.stderr_limit,
        StreamName::Stderr,
        Arc::clone(&capture),
    );

    let mut wait_error = None;
    let (status, mut stop) = match wait_or_stop(&mut child, spec, &capture, pid, process_group) {
        Ok(result) => result,
        Err(error) => {
            abort_spawned(&mut child, pid, process_group);
            wait_error = Some(error);
            (None, StopReason::Completed)
        }
    };

    // A fast writer can leave the cap flag unread when `try_wait` reports
    // completion. Readers are drained first so that completion never masks an
    // exceeded cap. The wait is bounded; a pipe held by an escaped descendant
    // cannot hang the runner.
    let readers = [stdout_done, stderr_done];
    let cleanup_deadline = Instant::now() + CLEANUP_TIMEOUT;
    let drain_deadline = (Instant::now() + READER_DRAIN_AFTER_EXIT).min(cleanup_deadline);
    let mut readers_done = await_readers(&readers, drain_deadline);
    if !readers_done {
        if process_group {
            signal_group(pid, "-TERM");
            thread::sleep(TERM_GRACE);
            signal_group(pid, "-KILL");
        }
        readers_done = await_readers(&readers, cleanup_deadline);
    }

    capture.closed.store(true, Ordering::SeqCst);
    if readers_done {
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();
    } else {
        drop(stdout_thread);
        drop(stderr_thread);
    }

    let stdout = capture.snapshot(StreamName::Stdout);
    let stderr = capture.snapshot(StreamName::Stderr);
    let stdout_bytes = capture.stdout_bytes.load(Ordering::SeqCst);
    let stderr_bytes = capture.stderr_bytes.load(Ordering::SeqCst);
    let exceeded = capture.exceeded_mask();
    let stdout_exceeded = exceeded & STDOUT_EXCEEDED != 0;
    let stderr_exceeded = exceeded & STDERR_EXCEEDED != 0;

    if stop == StopReason::Completed && exceeded != 0 {
        stop = StopReason::OutputLimit(stream_from_mask(exceeded));
        // The direct child is already reaped; terminate any descendants that
        // inherited the output pipes so the instance leaves no writers behind.
        if process_group {
            signal_group(pid, "-TERM");
            thread::sleep(TERM_GRACE);
            signal_group(pid, "-KILL");
        }
    }

    if let Some(error) = wait_error {
        return Err(error);
    }

    write_log(&stdout_log, &stdout)?;
    write_log(&stderr_log, &stderr)?;

    let (child_exit, signal) = status_parts(status);
    let elapsed_ms = started.elapsed().as_millis();
    Ok(RunReport {
        outcome: classify(stop, status),
        stdout,
        stderr,
        stdout_truncated: stdout_exceeded,
        stderr_truncated: stderr_exceeded,
        elapsed_ms,
        workdir,
        child_exit,
        signal,
        stdout_bytes,
        stderr_bytes,
        stdout_exceeded,
        stderr_exceeded,
        process_group,
        prlimit_applied,
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

fn reject_workdir_symlink(path: &Path, label: &str) -> Result<(), SetupError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SetupError::Workdir(format!(
            "refusing symlink {label}: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SetupError::Workdir(format!(
            "cannot inspect {}: {error}",
            path.display()
        ))),
    }
}

fn reject_log_target(path: &Path) -> Result<(), SetupError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SetupError::Log(format!(
            "refusing to follow symlink {}",
            path.display()
        ))),
        Ok(metadata) if !metadata.is_file() => Err(SetupError::Log(format!(
            "{} is not a regular file",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SetupError::Log(format!(
            "cannot inspect {}: {error}",
            path.display()
        ))),
    }
}

fn write_log(path: &Path, contents: &[u8]) -> Result<(), SetupError> {
    reject_log_target(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::Log(format!("{} has no parent directory", path.display())))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("log");
    for attempt in 0..16u32 {
        let temporary = parent.join(format!(".{name}.{}.{attempt}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let write_result = file.write_all(contents).and_then(|()| file.flush());
                drop(file);
                if let Err(error) = write_result {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(SetupError::Log(format!("{}: {error}", path.display())));
                }
                if let Err(error) = std::fs::rename(&temporary, path) {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(SetupError::Log(format!("{}: {error}", path.display())));
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(SetupError::Log(format!("{}: {error}", path.display())));
            }
        }
    }
    Err(SetupError::Log(format!(
        "{}: could not create a temporary log file",
        path.display()
    )))
}

fn build_command(
    spec: &RunSpec,
    workdir: &Path,
    use_setsid: bool,
    use_prlimit: bool,
) -> (Command, bool, bool) {
    let mut inner: Vec<OsString> = Vec::new();
    if use_prlimit {
        inner.push("prlimit".into());
        if let Some(value) = spec.limits.address_space {
            inner.push(format!("--as={value}").into());
        }
        if let Some(value) = spec.limits.cpu_seconds {
            inner.push(format!("--cpu={value}").into());
        }
        if let Some(value) = spec.limits.nofile {
            inner.push(format!("--nofile={value}").into());
        }
        if let Some(value) = spec.limits.nproc {
            inner.push(format!("--nproc={value}").into());
        }
        inner.push("--".into());
    }
    inner.push(spec.command.program.clone());
    inner.extend(spec.command.args.iter().cloned());

    // Process-group isolation and resource limits are independent wrappers:
    // `setsid` is optional, `prlimit` is required whenever limits are requested.
    let mut command = if use_setsid {
        let mut command = Command::new("setsid");
        command.arg("--");
        command.args(&inner);
        command
    } else {
        let mut command = Command::new(&inner[0]);
        command.args(&inner[1..]);
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
    (command, use_setsid, use_prlimit)
}

fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
    stream: StreamName,
    capture: Arc<Capture>,
) -> (thread::JoinHandle<()>, Receiver<()>) {
    let (done, finished) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    // Hold the buffer lock across the closed check and append so
                    // a final snapshot cannot race a late reader write.
                    let mut buffer = lock_recover(capture.buffer(stream));
                    if capture.closed.load(Ordering::SeqCst) {
                        break;
                    }
                    capture.bytes(stream).fetch_add(read, Ordering::SeqCst);
                    if limit == 0 {
                        buffer.extend_from_slice(&chunk[..read]);
                    } else {
                        let room = limit.saturating_sub(buffer.len());
                        let take = room.min(read);
                        buffer.extend_from_slice(&chunk[..take]);
                        if take < read {
                            capture.mark_exceeded(stream);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = done.send(());
    });
    (handle, finished)
}

fn await_readers(readers: &[Receiver<()>], deadline: Instant) -> bool {
    for reader in readers {
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        match reader.recv_timeout(deadline - now) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {}
            Err(RecvTimeoutError::Timeout) => return false,
        }
    }
    true
}

fn wait_or_stop(
    child: &mut Child,
    spec: &RunSpec,
    capture: &Capture,
    pid: u32,
    group_kill: bool,
) -> Result<(Option<ExitStatus>, StopReason), SetupError> {
    let wait_started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| SetupError::Wait(format!("try_wait: {error}")))?
        {
            return Ok((Some(status), StopReason::Completed));
        }
        if let Some(path) = &spec.cancel_file {
            if path.exists() {
                let status = stop_child(child, pid, group_kill)?;
                return Ok((status, StopReason::Cancelled));
            }
        }
        let exceeded = capture.exceeded_mask();
        if exceeded != 0 {
            let status = stop_child(child, pid, group_kill)?;
            return Ok((status, StopReason::OutputLimit(stream_from_mask(exceeded))));
        }
        if wait_started.elapsed() >= spec.timeout {
            let status = stop_child(child, pid, group_kill)?;
            return Ok((status, StopReason::Timeout));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn abort_spawned(child: &mut Child, pid: u32, group_kill: bool) {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    terminate(child, pid, group_kill);
    let _ = reap_bounded(child, deadline);
}

fn stop_child(
    child: &mut Child,
    pid: u32,
    group_kill: bool,
) -> Result<Option<ExitStatus>, SetupError> {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    terminate(child, pid, group_kill);
    reap_bounded(child, deadline)
}

fn reap_bounded(child: &mut Child, deadline: Instant) -> Result<Option<ExitStatus>, SetupError> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {}
            Err(error) => return Err(SetupError::Wait(format!("try_wait: {error}"))),
        }
        if Instant::now() >= deadline {
            return Err(SetupError::Wait(
                "child did not exit within the cleanup deadline".to_string(),
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(unix)]
fn terminate(child: &mut Child, pid: u32, group_kill: bool) {
    if group_kill {
        signal_group(pid, "-TERM");
        thread::sleep(TERM_GRACE);
        signal_group(pid, "-KILL");
    }
    // Also target the direct child in case session creation failed or the
    // process group is no longer addressable.
    let _ = child.kill();
}

#[cfg(not(unix))]
fn terminate(child: &mut Child, _pid: u32, _group_kill: bool) {
    let _ = child.kill();
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: &str) {
    let target = format!("-{pid}");
    for program in ["/bin/kill", "/usr/bin/kill", "kill"] {
        let status = Command::new(program)
            .args([signal, "--", target.as_str()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            return;
        }
    }
}

fn stream_from_mask(mask: u8) -> StreamName {
    if mask & STDOUT_EXCEEDED != 0 {
        StreamName::Stdout
    } else {
        StreamName::Stderr
    }
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
fn status_parts(status: Option<ExitStatus>) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    match status {
        Some(status) => (status.code(), status.signal()),
        None => (None, None),
    }
}

#[cfg(not(unix))]
fn status_parts(status: Option<ExitStatus>) -> (Option<i32>, Option<i32>) {
    (status.and_then(|status| status.code()), None)
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
