use cc_silicon_compiler::codec::{hex32, sha256};
use cc_silicon_compiler::target::{
    CompilerConfig, ConfigError, DataModel, Endianness, ObjectFormat, ProbeError, ProbeReport,
    ProbeStatus, ScalarKind, TargetSpec, VerificationState, MEASURED_SCALARS,
    REQUIRED_PROBE_FIELDS,
};

const PROBE: &str = include_str!("../contracts/target/aarch64-linux-probe.txt");

fn proposed_value(target: &TargetSpec, key: &str) -> String {
    let profile = target.profile();
    match key {
        "triple" => profile.triple.to_owned(),
        "object_format" => profile.object_format.name().to_owned(),
        "endianness" => profile.endianness.name().to_owned(),
        "data_model" => profile.data_model.name().to_owned(),
        "char.signed" => target.scalar(ScalarKind::Char).signed.unwrap().to_string(),
        "wchar_t.signed" => target
            .scalar(ScalarKind::WCharT)
            .signed
            .unwrap()
            .to_string(),
        "wchar_t.encoding" => target.wchar_encoding().name().to_owned(),
        "longdouble.format" => target
            .scalar(ScalarKind::LongDouble)
            .float
            .unwrap()
            .name()
            .to_owned(),
        "abi.gp_arg_regs" => target.abi().gp_arg_regs.to_string(),
        "abi.fp_arg_regs" => target.abi().fp_arg_regs.to_string(),
        "abi.stack_align" => target.abi().stack_align.to_string(),
        "abi.variadic_register_save_area" => target.abi().variadic_register_save_area.to_string(),
        other => {
            if let Some(name) = other.strip_prefix("sizeof.") {
                let kind = measured_kind(name).expect("known measured kind");
                target.scalar(kind).size.to_string()
            } else if let Some(name) = other.strip_prefix("alignof.") {
                let kind = measured_kind(name).expect("known measured kind");
                target.scalar(kind).align.to_string()
            } else {
                panic!("unmapped probe field `{other}`")
            }
        }
    }
}

fn measured_kind(probe_name: &str) -> Option<ScalarKind> {
    MEASURED_SCALARS
        .into_iter()
        .find(|kind| kind.probe_name() == probe_name)
}

fn signed_report() -> ProbeReport {
    let mut report = ProbeReport::parse(PROBE).unwrap();
    report.set("verified", "true");
    let hash = hex32(&sha256(&report.unsigned_body()));
    report.set("report_hash", &hash);
    report
}

#[test]
fn target_profile_identity_is_frozen() {
    let target = TargetSpec::aarch64_unknown_linux_gnu_unverified();
    let profile = target.profile();
    assert_eq!(profile.triple, "aarch64-unknown-linux-gnu");
    assert_eq!(profile.object_format, ObjectFormat::Elf);
    assert_eq!(profile.data_model, DataModel::Lp64);
    assert_eq!(profile.endianness, Endianness::Little);
    assert_eq!(profile.abi_name, "AAPCS64");
}

#[test]
fn concrete_values_are_unverified_until_probed() {
    let target = TargetSpec::aarch64_unknown_linux_gnu_unverified();
    assert!(!target.verification().is_verified());
    assert_eq!(target.verification().report_hash(), None);
    assert_eq!(target.verification(), VerificationState::UNVERIFIED);
    assert!(matches!(
        target.require_verified(),
        Err(ConfigError::TargetUnverified { .. })
    ));

    let config = CompilerConfig::default();
    assert!(matches!(
        config.ensure_codegen_ready(),
        Err(ConfigError::TargetUnverified { .. })
    ));
}

#[test]
fn no_darwin_or_macos_values_are_present() {
    let body: String = PROBE
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    for forbidden in ["darwin", "macos", "x86_64", "ilp32", "mach-o"] {
        assert!(
            !body.contains(forbidden),
            "probe fixture must not contain `{forbidden}`"
        );
    }
    let target = TargetSpec::aarch64_unknown_linux_gnu_unverified();
    assert!(target.abi().variadic_register_save_area);
}

#[test]
fn probe_fixture_is_unverified_and_matches_every_required_field() {
    let report = ProbeReport::parse(PROBE).unwrap();
    assert!(report.require_fields(REQUIRED_PROBE_FIELDS).is_ok());
    assert!(matches!(
        report.verify_verified(),
        Err(ProbeError::NotVerified)
    ));
    assert!(matches!(report.status(), Err(ProbeError::NotVerified)));

    let target = TargetSpec::aarch64_unknown_linux_gnu_unverified();
    for key in REQUIRED_PROBE_FIELDS {
        let actual = report
            .get(key)
            .unwrap_or_else(|| panic!("missing probe field `{key}`"));
        assert_eq!(
            actual,
            proposed_value(&target, key).as_str(),
            "probe field `{key}` disagrees with the proposal"
        );
    }
}

#[test]
fn attested_report_produces_a_verified_target() {
    let report = signed_report();
    assert_eq!(
        report.status().unwrap(),
        ProbeStatus::Verified {
            report_hash: sha256(&report.unsigned_body())
        }
    );
    let target = TargetSpec::attest(&report).unwrap();
    assert!(target.verification().is_verified());
    assert_eq!(
        target.verification().report_hash(),
        Some(sha256(&report.unsigned_body()))
    );
    assert!(target.require_verified().is_ok());
}

#[test]
fn attestation_rejects_tampered_and_incomplete_reports() {
    let mut tampered = signed_report();
    tampered.set("sizeof.long", "4");
    assert!(matches!(
        TargetSpec::attest(&tampered),
        Err(ProbeError::HashMismatch { .. })
    ));

    let missing_text: String = PROBE
        .lines()
        .filter(|line| !line.trim_start().starts_with("sizeof.long "))
        .collect::<Vec<_>>()
        .join("\n");
    let mut missing = ProbeReport::parse(&missing_text).unwrap();
    missing.set("verified", "true");
    let hash = hex32(&sha256(&missing.unsigned_body()));
    missing.set("report_hash", &hash);
    assert!(matches!(
        TargetSpec::attest(&missing),
        Err(ProbeError::MissingField {
            field: "sizeof.long"
        })
    ));

    let mut wrong_identity = signed_report();
    wrong_identity.set("triple", "x86_64-unknown-linux-gnu");
    let hash = hex32(&sha256(&wrong_identity.unsigned_body()));
    wrong_identity.set("report_hash", &hash);
    assert!(matches!(
        TargetSpec::attest(&wrong_identity),
        Err(ProbeError::IdentityMismatch {
            field: "triple",
            ..
        })
    ));
}

#[test]
fn attestation_rejects_an_unknown_wchar_encoding() {
    let mut report = signed_report();
    report.set("wchar_t.encoding", "ebcdic");
    let hash = hex32(&sha256(&report.unsigned_body()));
    report.set("report_hash", &hash);
    assert!(matches!(
        TargetSpec::attest(&report),
        Err(ProbeError::BadValue {
            field,
            ..
        }) if field == "wchar_t.encoding"
    ));
}
