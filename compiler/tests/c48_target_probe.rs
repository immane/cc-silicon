// ============================================================================
// c48_target_probe.rs — H01/H07 real-probe attestation regression.
//
// The fixture is an H07-classified H01 probe report captured from a pinned
// Debian 12 aarch64 container (GCC 12.2.0). It proves the target model can be
// attested from a real report and that tampering is rejected. It is NOT a
// claim that the official T00 reference baseline, bare-metal CI substrate, or
// GCC torture corpus has been provisioned.
// ============================================================================

use cc_silicon_compiler::target::{ProbeReport, ScalarKind, TargetSpec, WideCharEncoding};

const ATTESTED_REPORT: &str =
    include_str!("fixtures/target-probe/aarch64-debian12-gcc12.2.0-attested.txt");

#[test]
fn classified_probe_report_attests_the_target() {
    let report = ProbeReport::parse(ATTESTED_REPORT).expect("probe report parses");
    let target = TargetSpec::attest(&report).expect("classified probe report attests");
    assert!(target.verification().is_verified());
    assert!(target.require_verified().is_ok());

    assert_eq!(target.abi().name, "AAPCS64");
    assert_eq!(target.abi().gp_arg_regs, 8);
    assert_eq!(target.abi().fp_arg_regs, 8);
    assert_eq!(target.abi().stack_align, 16);
    assert!(target.abi().variadic_register_save_area);

    assert_eq!(target.scalar(ScalarKind::Int).size, 4);
    assert_eq!(target.scalar(ScalarKind::Long).size, 8);
    assert_eq!(target.scalar(ScalarKind::Pointer).size, 8);
    assert_eq!(target.scalar(ScalarKind::LongDouble).size, 16);
    assert_eq!(target.wchar_encoding(), WideCharEncoding::Utf32);
}

#[test]
fn attested_report_tampering_is_rejected() {
    let tampered = ATTESTED_REPORT.replace("sizeof.int = 4", "sizeof.int = 8");
    assert_ne!(tampered, ATTESTED_REPORT);
    let report = ProbeReport::parse(&tampered).expect("tampered report still parses");
    assert!(TargetSpec::attest(&report).is_err());
}
