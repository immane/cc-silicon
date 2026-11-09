// ============================================================================
// sprint_h06.rs — H05 runner symlink containment and victim preservation.
//
// Covers the audit finding that log writes follow symlinks. Every fixture is
// created below an isolated temporary root; no repository or unrelated host
// file is touched.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cc_silicon_runner::{
    run, CommandSpec, ResourceLimits, RunOutcome, RunReport, RunSpec, SetupError,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "h06-runner-{}-{test}-{unique}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn workdir(&self) -> PathBuf {
        self.root.join("workdir")
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

fn spec(workdir: PathBuf, script: &str) -> RunSpec {
    RunSpec {
        workdir,
        timeout: Duration::from_millis(2_000),
        stdout_limit: 1024,
        stderr_limit: 1024,
        cancel_file: None,
        limits: ResourceLimits::default(),
        command: CommandSpec {
            program: OsString::from("sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
        },
    }
}

// The exact error constructor is not frozen: both log and workdir setup
// failures are acceptable when a runner path is unsafe. What matters is that
// the run is rejected and the symlink target is untouched.
fn assert_setup_rejection(result: Result<RunReport, SetupError>) {
    match result {
        Err(SetupError::Workdir(_)) | Err(SetupError::Log(_)) => {}
        Err(other) => panic!("expected a workdir/log setup rejection, got {other:?}"),
        Ok(report) => panic!(
            "a symlinked runner path must be rejected, got outcome {:?}",
            report.outcome
        ),
    }
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

#[test]
fn stdout_log_symlink_is_rejected_before_the_child_runs() {
    let scratch = Scratch::new("stdout-log-symlink");
    let workdir = scratch.workdir();
    std::fs::create_dir_all(&workdir).expect("workdir");

    let victim = scratch.path("victim-stdout.txt");
    std::fs::write(&victim, b"victim stdout bytes").expect("victim file");
    // Relative target outside the workdir: following it would overwrite an
    // unrelated file.
    symlink("../victim-stdout.txt", workdir.join("stdout.log")).expect("stdout symlink");

    let marker = workdir.join("child-ran");
    let result = run(&spec(workdir, "printf child-output; touch child-ran"));

    assert_setup_rejection(result);
    assert_eq!(read(&victim), b"victim stdout bytes");
    assert!(
        !marker.exists(),
        "the child executed even though the log symlink was rejected"
    );
}

#[test]
fn stderr_log_symlink_is_rejected_and_victim_preserved() {
    let scratch = Scratch::new("stderr-log-symlink");
    let workdir = scratch.workdir();
    std::fs::create_dir_all(&workdir).expect("workdir");

    let victim = scratch.path("victim-stderr.txt");
    std::fs::write(&victim, b"victim stderr bytes").expect("victim file");
    symlink(&victim, workdir.join("stderr.log")).expect("stderr symlink");

    let result = run(&spec(workdir, "printf child-error >&2"));

    assert_setup_rejection(result);
    assert_eq!(read(&victim), b"victim stderr bytes");
}

#[test]
fn tmp_symlink_is_rejected_and_victim_directory_preserved() {
    let scratch = Scratch::new("tmp-symlink");
    let workdir = scratch.workdir();
    std::fs::create_dir_all(&workdir).expect("workdir");

    let victim_dir = scratch.path("victim-tmp");
    std::fs::create_dir_all(&victim_dir).expect("victim dir");
    let sentinel = victim_dir.join("sentinel");
    std::fs::write(&sentinel, b"victim tmp sentinel").expect("sentinel");
    symlink("../victim-tmp", workdir.join("tmp")).expect("tmp symlink");

    let result = run(&spec(
        workdir,
        "printf child-write > \"$TMPDIR/child-write\"",
    ));

    assert_setup_rejection(result);
    assert_eq!(read(&sentinel), b"victim tmp sentinel");
    assert!(
        !victim_dir.join("child-write").exists(),
        "the child wrote through the tmp symlink into the victim directory"
    );
}

#[test]
fn ordinary_workdir_files_and_link_targets_survive_a_run() {
    let scratch = Scratch::new("preservation");
    let workdir = scratch.workdir();
    std::fs::create_dir_all(workdir.join("nested")).expect("workdir");

    std::fs::write(workdir.join("keep.txt"), b"keep me").expect("keep file");
    std::fs::write(workdir.join("nested/keep2.txt"), b"nested keep").expect("nested keep");
    let victim = scratch.path("shared-victim.txt");
    std::fs::write(&victim, b"shared victim").expect("victim file");
    symlink(&victim, workdir.join("input-link")).expect("input symlink");

    let report = run(&spec(workdir.clone(), "printf out; printf err >&2")).expect("runner");

    assert_eq!(report.outcome, RunOutcome::Completed { code: 0 });
    assert_eq!(read(&workdir.join("keep.txt")), b"keep me");
    assert_eq!(read(&workdir.join("nested/keep2.txt")), b"nested keep");
    assert_eq!(read(&victim), b"shared victim");
    assert!(
        std::fs::symlink_metadata(workdir.join("input-link"))
            .expect("link metadata")
            .file_type()
            .is_symlink(),
        "the runner replaced an unrelated workdir symlink"
    );
    assert_eq!(read(&workdir.join("stdout.log")), b"out");
    assert_eq!(read(&workdir.join("stderr.log")), b"err");
}
