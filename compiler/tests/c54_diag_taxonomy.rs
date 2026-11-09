// ============================================================================
// c54_diag_taxonomy.rs — `/47` diagnostic taxonomy acceptance.
//
// Pins the appended group inventory, the named code registry, uniqueness, and
// the new structured-helpers. It does not claim migration of every legacy
// `Task:4` call site.
// ============================================================================

use cc_silicon_compiler::contract::DIAG_GROUP_NAMES;
use cc_silicon_compiler::diagnostic::{
    DiagGroup, DiagnosticCode, DiagnosticDraft, DIAGNOSTIC_CODES,
};
use std::collections::BTreeSet;

#[test]
fn diagnostic_groups_append_lex_parse_type_const_ir_codegen_host() {
    assert_eq!(
        &DIAG_GROUP_NAMES[..9],
        &[
            "protocol",
            "arena",
            "config",
            "target",
            "manifest",
            "task",
            "semantic",
            "unsupported",
            "internal",
        ]
    );
    assert_eq!(
        &DIAG_GROUP_NAMES[9..],
        &["lex", "parse", "type", "const", "ir", "codegen", "host"]
    );
}

#[test]
fn code_registry_names_and_codes_are_unique() {
    let mut names = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for spec in DIAGNOSTIC_CODES {
        assert!(names.insert(spec.name), "duplicate name {}", spec.name);
        assert!(
            codes.insert((spec.code.group.name(), spec.code.code)),
            "duplicate code {}:{}",
            spec.code.group.name(),
            spec.code.code
        );
    }
    assert_eq!(
        DiagnosticCode::TASK_MISSING_RECORD,
        DiagnosticCode::new(DiagGroup::Task, 5)
    );
    assert_eq!(
        DiagnosticCode::LEX_INVALID_TOKEN,
        DiagnosticCode::new(DiagGroup::Lex, 1)
    );
    assert_eq!(
        DiagnosticCode::HOST_TIMEOUT,
        DiagnosticCode::new(DiagGroup::Host, 3)
    );
}

#[test]
fn structured_helpers_use_the_named_codes() {
    let missing = DiagnosticDraft::missing_record("missing record");
    assert_eq!(missing.code, DiagnosticCode::TASK_MISSING_RECORD);
    let parse = DiagnosticDraft::parse_unexpected("unexpected token");
    assert_eq!(parse.code, DiagnosticCode::PARSE_UNEXPECTED_TOKEN);
    let ty = DiagnosticDraft::type_mismatch("incompatible");
    assert_eq!(ty.code, DiagnosticCode::TYPE_INCOMPATIBLE);
    let ir = DiagnosticDraft::ir_invalid("bad cfg");
    assert_eq!(ir.code, DiagnosticCode::IR_INVALID_SHAPE);
    let host = DiagnosticDraft::host_io("read failed");
    assert_eq!(host.code, DiagnosticCode::HOST_IO);
}
