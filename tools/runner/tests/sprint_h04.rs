// ============================================================================
// sprint_h04.rs — H05 lifecycle regressions: descendant-held pipes, TERM
// resistance, and the total deadline for timeout cleanup.
//
// Each probe invokes the runner library in a worker thread and waits with an
// outer wall-clock deadline. That outer bound is deliberate: the runner under
// test cannot supervise a hang in its own cleanup path, so the harness must
// report the failure rather than letting the test process hang. Child scripts
// are finite (short sleeps) so a failed probe cannot leak an endless process.
// ============================================================================

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use cc_silicon_runner::{run, CommandSpec, ResourceLimits, RunOutcome, RunSpec};

/// Hard limit for one complete probe, including runner setup, timeout,
/// termination, waits, and log persistence.
const OUTER_DEADLINE: Duration = Duration::from_secs(4);

/// A correctly bounded probe should return far sooner than this. The child
/// scripts keep their descendants alive for eight seconds, so leaking a
/// descendant-held pipe is distinguishable from ordinary scheduler jitter.
const RETURN_DEADLINE: Duration = Duration::from_secs(3);

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "h04-runner-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create test workdir");
    dir
}

fn shell_spec(dir: PathBuf, timeout_ms: u64, script: &str) -> RunSpec {
    RunSpec {
        workdir: dir,
        timeout: Duration::from_millis(timeout_ms),
        stdout_limit: 4096,
        stderr_limit: 4096,
        cancel_file: None,
        limits: ResourceLimits::default(),
        command: CommandSpec {
            program: OsString::from("sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
        },
    }
}

fn within_outer_deadline<T, F>(label: &'static str, limit: Duration, body: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let _ = sender.send(body());
    });
    match receiver.recv_timeout(limit) {
        Ok(result) => {
            worker.join().expect("outer worker thread");
            result
        }
        Err(RecvTimeoutError::Timeout) => {
            // Do not join here: that would repeat the unbounded wait under test.
            panic!(
                "{label}: run() did not return within the outer {limit:?} deadline; \
                 post-exit pipe readers or termination cleanup are not bounded"
            );
        }
        Err(RecvTimeoutError::Disconnected) => {
            worker.join().expect("outer worker thread");
            panic!("{label}: outer worker exited without a result");
        }
    }
}

/// The direct child exits immediately, but a background descendant inherits
/// the captured stdout/stderr pipes and stays alive. The completed child's
/// status must be reported promptly; its descendant must not extend cleanup
/// for eight seconds.
#[test]
fn descendant_held_pipes_do_not_block_completed_return() {
    let dir = workdir("descendant-pipes");
    let run_spec = shell_spec(dir, 1_000, "sleep 8 & printf ready; exit 3");
    let started = Instant::now();
    let report = within_outer_deadline("descendant-held pipes", OUTER_DEADLINE, move || {
        run(&run_spec)
    })
    .expect("runner setup");
    let elapsed = started.elapsed();

    assert!(
        elapsed < RETURN_DEADLINE,
        "the direct child exited immediately, but run() returned after {elapsed:?}; \
         a descendant holding the capture pipes extended cleanup"
    );
    assert_eq!(report.outcome, RunOutcome::Completed { code: 3 });
    assert_eq!(report.stdout, b"ready".to_vec());
    assert_eq!(
        std::fs::read(report.workdir.join("stdout.log")).expect("stdout log"),
        b"ready".to_vec()
    );
}

/// The child ignores SIGTERM for its whole lifetime. The supervisor must
/// escalate rather than wait for the eight-second sleep to finish, and the
/// reported outcome must remain a timeout (not a normal completion).
#[test]
fn term_resistant_child_is_still_stopped_by_timeout() {
    let dir = workdir("term-resistant");
    let script = "trap '' TERM\nsleep 8 >/dev/null 2>&1\nexit 0";
    let run_spec = shell_spec(dir, 200, script);
    let started = Instant::now();
    let report = within_outer_deadline("TERM-resistant child", OUTER_DEADLINE, move || {
        run(&run_spec)
    })
    .expect("runner setup");
    let elapsed = started.elapsed();

    assert_eq!(report.outcome, RunOutcome::Timeout);
    assert_eq!(report.outcome.exit_code(), 3);
    assert!(
        elapsed < RETURN_DEADLINE,
        "timeout termination of a TERM-resistant child took {elapsed:?}"
    );
}

/// Timeout is a total deadline, not only the moment the first signal is sent.
/// Here both the direct child and its background descendant ignore SIGTERM and
/// the descendant keeps the capture pipes open. The run must still return a
/// timeout inside the outer bound instead of blocking in wait/join cleanup.
#[test]
fn total_deadline_bounds_timeout_with_held_pipes() {
    let dir = workdir("total-deadline");
    let script =
        "trap '' TERM\n( trap '' TERM; sleep 8 ) &\nprintf started\nsleep 8 >/dev/null 2>&1\n";
    let run_spec = shell_spec(dir, 200, script);
    let started = Instant::now();
    let report = within_outer_deadline("total deadline", OUTER_DEADLINE, move || run(&run_spec))
        .expect("runner setup");
    let elapsed = started.elapsed();

    assert_eq!(report.outcome, RunOutcome::Timeout);
    assert_eq!(report.outcome.exit_code(), 3);
    assert!(
        elapsed < RETURN_DEADLINE,
        "timeout plus descendant cleanup took {elapsed:?}; the total deadline was not bounded"
    );
}
