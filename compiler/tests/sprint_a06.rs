// ============================================================================
// sprint_a06.rs — independent ABI evidence for full modeled results versus
// low-eight-bit process exit status.
//
// The modeled signed-`int` result is the primary oracle. `--interpret-ir`
// must report the exact sign and minimal big-endian magnitude bytes, including
// values whose high bits are invisible to a Unix process exit status. The
// cases are deliberately grouped so different modeled values share the same
// low byte: deriving the model from the exit status would not distinguish
// `254` from `-2`, or `511` from `-1`, and `256`/`-256` both exit zero.
//
// Every expected magnitude, sign, and exit byte below is hand-derived from
// the signed-32-bit source subset. On Linux/AArch64 the emitted assembly is
// assembled and linked by the host GCC and run under a finite deadline; GCC
// never compiles candidate C source. Other hosts assert only the modeled
// result and do not claim a native pass.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static BOUNDED_RUN_ID: AtomicU64 = AtomicU64::new(0);

struct Case {
    name: &'static str,
    source: &'static str,
    /// Minimal big-endian magnitude bytes as printed after `value=`.
    magnitude_hex: &'static str,
    negative: bool,
    /// Low eight bits of the modeled two's-complement i32 value.
    exit_low_byte: u8,
}

// A source `0` case is deliberately absent: the `/55` literal-fold subset
// still treats zero as octal, so zero-source coverage belongs to the L-series
// prerequisite rather than this ABI test.
const CASES: &[Case] = &[
    Case {
        name: "positive-254",
        source: "int main(void) { return 254; }\n",
        magnitude_hex: "fe",
        negative: false,
        exit_low_byte: 254,
    },
    Case {
        name: "negative-2",
        source: "int main(void) { return -2; }\n",
        magnitude_hex: "02",
        negative: true,
        exit_low_byte: 254,
    },
    Case {
        name: "wide-256",
        source: "int main(void) { return 256; }\n",
        magnitude_hex: "0100",
        negative: false,
        exit_low_byte: 0,
    },
    Case {
        name: "negative-256",
        source: "int main(void) { return -256; }\n",
        magnitude_hex: "0100",
        negative: true,
        exit_low_byte: 0,
    },
    Case {
        name: "wide-257",
        source: "int main(void) { return 257; }\n",
        magnitude_hex: "0101",
        negative: false,
        exit_low_byte: 1,
    },
    Case {
        name: "negative-255",
        source: "int main(void) { return -255; }\n",
        magnitude_hex: "ff",
        negative: true,
        exit_low_byte: 1,
    },
    Case {
        name: "wide-300",
        source: "int main(void) { return 300; }\n",
        magnitude_hex: "012c",
        negative: false,
        exit_low_byte: 44,
    },
    Case {
        name: "negative-300",
        source: "int main(void) { return -300; }\n",
        magnitude_hex: "012c",
        negative: true,
        exit_low_byte: 212,
    },
    Case {
        name: "wide-511",
        source: "int main(void) { return 511; }\n",
        magnitude_hex: "01ff",
        negative: false,
        exit_low_byte: 255,
    },
    Case {
        name: "negative-one",
        source: "int main(void) { return -1; }\n",
        magnitude_hex: "01",
        negative: true,
        exit_low_byte: 255,
    },
    Case {
        name: "wide-65535",
        source: "int main(void) { return 65535; }\n",
        magnitude_hex: "ffff",
        negative: false,
        exit_low_byte: 255,
    },
    Case {
        name: "negative-65535",
        source: "int main(void) { return -65535; }\n",
        magnitude_hex: "ffff",
        negative: true,
        exit_low_byte: 1,
    },
    Case {
        name: "wide-65536",
        source: "int main(void) { return 65536; }\n",
        magnitude_hex: "010000",
        negative: false,
        exit_low_byte: 0,
    },
    Case {
        name: "wide-max-i32",
        source: "int main(void) { return 2147483647; }\n",
        magnitude_hex: "7fffffff",
        negative: false,
        exit_low_byte: 255,
    },
    Case {
        name: "minimum-i32",
        // -2147483647 - 1 = -2147483648, representable without overflow.
        source: "int main(void) { return -2147483647 - 1; }\n",
        magnitude_hex: "80000000",
        negative: true,
        exit_low_byte: 0,
    },
];

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos()
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-a06-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Wait for a child with a finite deadline. Standard Rust has no built-in
/// command timeout, so output is redirected to files and the child is killed
/// when the bound expires.
fn bounded_output(command: &mut Command, limit: Duration, label: &str) -> Output {
    let serial = BOUNDED_RUN_ID.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "sprint-a06-bound-{}-{}-{}-{}",
        std::process::id(),
        serial,
        label,
        unique_suffix()
    ));
    let stdout_path = base.with_extension("stdout");
    let stderr_path = base.with_extension("stderr");
    let stdout_file = fs::File::create(&stdout_path).expect("bounded stdout file");
    let stderr_file = fs::File::create(&stderr_path).expect("bounded stderr file");
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{label} failed to spawn: {error}"));
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait().expect("bounded child wait") {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
                let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                panic!("{label} exceeded {limit:?}\nstdout: {stdout}\nstderr: {stderr}");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let stdout = fs::read(&stdout_path).expect("bounded stdout");
    let stderr = fs::read(&stderr_path).expect("bounded stderr");
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Output {
        status,
        stdout,
        stderr,
    }
}

fn run_candidate(dir: &Path, source: &str, asm: Option<&Path>) -> Output {
    fs::create_dir_all(dir).expect("source dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    let mut command = Command::new(binary());
    if let Some(path) = asm {
        command.arg("--emit-asm").arg(path);
    }
    command.arg("--interpret-ir").arg(&source_path);
    bounded_output(&mut command, Duration::from_secs(60), "candidate-a06")
}

/// Low eight bits of the modeled two's-complement i32 value.
fn expected_exit_byte(case: &Case) -> u8 {
    let magnitude = u64::from_str_radix(case.magnitude_hex, 16)
        .expect("case table magnitude must be hexadecimal");
    let low = (magnitude & 0xff) as u8;
    if case.negative {
        low.wrapping_neg()
    } else {
        low
    }
}

/// Parse the single modeled-result line into its exact magnitude and sign.
fn modeled_result(stdout: &[u8], context: &str) -> (String, bool) {
    let text = String::from_utf8_lossy(stdout);
    let mut lines = text.lines().filter(|line| line.starts_with("interpret: "));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("{context}: missing interpret line\nstdout: {text}"));
    assert!(
        lines.next().is_none(),
        "{context}: multiple interpret lines\nstdout: {text}"
    );
    let rest = line
        .strip_prefix("interpret: value=")
        .unwrap_or_else(|| panic!("{context}: malformed interpret line: {line}"));
    let (magnitude, negative) = rest
        .split_once(" negative=")
        .unwrap_or_else(|| panic!("{context}: malformed interpret line: {line}"));
    (magnitude.to_string(), negative == "true")
}

fn assert_modeled(output: &Output, case: &Case) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}: candidate failed\nstdout: {}\nstderr: {}",
        case.name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (magnitude, negative) = modeled_result(&output.stdout, case.name);
    assert_eq!(
        (magnitude.as_str(), negative),
        (case.magnitude_hex, case.negative),
        "{}: full modeled result differs\nstdout: {}\nstderr: {}",
        case.name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn modeled_full_values_are_reported_independently_of_exit_bytes() {
    for case in CASES {
        assert_eq!(
            case.exit_low_byte,
            expected_exit_byte(case),
            "{}: hand-written exit byte disagrees with the modeled value",
            case.name
        );
        let dir = workdir(&format!("model-{}", case.name));
        let output = run_candidate(&dir, case.source, None);
        assert_modeled(&output, case);
    }
}

#[cfg(target_os = "linux")]
fn aarch64_gcc() -> Option<&'static str> {
    let output = match Command::new("gcc").arg("-dumpmachine").output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("SKIP: native AArch64 execution requires gcc: {error}");
            return None;
        }
    };
    if !output.status.success() {
        eprintln!(
            "SKIP: gcc -dumpmachine failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    let machine = String::from_utf8_lossy(&output.stdout);
    let machine = machine.trim();
    if !machine.starts_with("aarch64") || machine.contains("aarch64_be") {
        eprintln!(
            "SKIP: native AArch64 execution requires an aarch64-linux gcc; found `{machine}`"
        );
        return None;
    }
    Some("gcc")
}

#[cfg(target_os = "linux")]
#[test]
fn native_exit_status_is_only_the_low_eight_bits() {
    let Some(gcc) = aarch64_gcc() else {
        return;
    };
    for case in CASES {
        let dir = workdir(&format!("native-{}", case.name));
        let asm = dir.join("main.s");
        let exe = dir.join("main");
        let output = run_candidate(&dir, case.source, Some(&asm));
        assert_modeled(&output, case);

        let mut build = Command::new(gcc);
        build.arg("-o").arg(&exe).arg(&asm);
        let build = bounded_output(&mut build, Duration::from_secs(60), "gcc-link-a06");
        assert!(
            build.status.success(),
            "{}: external GCC failed to assemble/link emitted assembly\nstdout: {}\nstderr: {}",
            case.name,
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let mut run = Command::new(&exe);
        let run = bounded_output(&mut run, Duration::from_secs(30), "native-run-a06");
        assert_eq!(
            run.status.code(),
            Some(i32::from(case.exit_low_byte)),
            "{}: native exit differs from the low byte of the modeled value {}{}\nstdout: {}\nstderr: {}",
            case.name,
            if case.negative { "-" } else { "" },
            case.magnitude_hex,
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }
}
