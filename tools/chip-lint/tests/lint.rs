use cc_silicon_chip_lint::lint_source;

fn messages(source: &str) -> Vec<String> {
    lint_source("fixture.rs", source)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn accepts_restricted_unit_chip() {
    let diagnostics = messages(
        r#"
        struct AddChip;
        impl RestrictedChip for AddChip {
            type Input = u32;
            type Output = u32;
            fn compute(&self, input: &u32) -> u32 { input.wrapping_add(1) }
        }
        "#,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn accepts_supported_chip_macro() {
    let diagnostics = messages(
        r#"
        silicon_chip! {
            struct AddChip;
            impl RestrictedChip for AddChip {
                type Input = u32;
                type Output = u32;
                fn compute(&self, input: &Self::Input) -> Self::Output {
                    input.wrapping_add(1)
                }
            }
        }
        "#,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn rejects_stateful_chip_struct() {
    let diagnostics = messages("struct StatefulChip { value: u32 }");
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("unit structs")));
}

#[test]
fn rejects_legacy_bus_mutating_trait() {
    let diagnostics = messages(
        r#"
        impl LogicChip<MyBus> for OldChip {
            fn tick(&self, pins: &Pins, bus: &mut MyBus) {}
        }
        "#,
    );
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("full mutable bus")));
}

#[test]
fn rejects_host_and_nondeterministic_imports() {
    let diagnostics = messages("use std::time::Instant;");
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("Host or nondeterministic")));

    let source = r#"
        struct EnvChip;
        impl RestrictedChip for EnvChip {
            type Input = ();
            type Output = ();
            fn compute(&self, _: &()) { let _ = std::env::var("SECRET"); }
        }
    "#;
    let diagnostics = messages(source);
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("Host or nondeterministic")));
}

#[test]
fn rejects_opaque_macro_and_indirect_calls() {
    let source = r#"
        struct MacroChip;
        impl RestrictedChip for MacroChip {
            type Input = u32;
            type Output = u32;
            fn compute(&self, input: &u32) -> u32 { helper!(input) }
        }
    "#;
    let diagnostics = messages(source);
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("macros inside compute")));

    let source = r#"
        struct IndirectChip;
        impl RestrictedChip for IndirectChip {
            type Input = fn() -> u32;
            type Output = u32;
            fn compute(&self, input: &Self::Input) -> u32 { input() }
        }
    "#;
    let diagnostics = messages(source);
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("not statically auditable")));
}

#[test]
fn rejects_cross_chip_and_unsafe_calls() {
    let source = r#"
        struct OtherChip;
        struct CallingChip;
        impl RestrictedChip for CallingChip {
            type Input = ();
            type Output = ();
            fn compute(&self, _: &()) { OtherChip.tick() }
        }
    "#;
    let diagnostics = messages(source);
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("chip-to-chip")));

    let source = r#"
        struct UnsafeChip;
        impl RestrictedChip for UnsafeChip {
            type Input = ();
            type Output = ();
            fn compute(&self, _: &()) { unsafe { core::hint::unreachable_unchecked() } }
        }
    "#;
    let diagnostics = messages(source);
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("unsafe blocks")));
}

#[test]
fn parse_failure_is_a_lint_failure() {
    let diagnostics = messages("impl RestrictedChip for Broken { fn compute(");
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("refusing strict lint"));
}

#[test]
fn accepts_deterministic_btree_and_self_rooted_reexport() {
    assert!(messages("use std::collections::BTreeMap;").is_empty());
    assert!(messages("use std::collections::BTreeSet;").is_empty());
    assert!(messages("pub use self::fold::{FoldChip};").is_empty());
    // A bare module root stays rejected so cross-chip imports are visible.
    assert!(!messages("pub use fold::{FoldChip};").is_empty());
}

#[test]
fn accepts_transparent_matches_macro() {
    let source = r#"
        struct MatchChip;
        impl MatchChip {
            fn compute(&self, kind: u32) -> bool {
                matches!(kind, 1 | 2 | 3)
            }
        }
    "#;
    assert!(messages(source).is_empty());
}

#[test]
fn rejects_worker_compute_host_api() {
    let source = r#"
        struct FoldChip;
        impl FoldChip {
            fn compute(&self) { let _ = std::env::var("SECRET"); }
        }
    "#;
    let diagnostics = messages(source);
    assert!(diagnostics
        .iter()
        .any(|message| message.contains("Host or nondeterministic")));
}

#[test]
fn accepts_type_constructors_but_rejects_bare_unknown_calls() {
    let source = r#"
        struct TokenChip;
        impl TokenChip {
            fn compute(&self) -> u32 {
                let handle = DraftRef(0);
                let ok: Option<u32> = Some(1);
                handle.0 + ok.unwrap_or(0)
            }
        }
    "#;
    assert!(messages(source).is_empty());

    let source = r#"
        struct UnknownCallChip;
        impl UnknownCallChip {
            fn compute(&self) -> u32 {
                mystery_helper(1)
            }
        }
    "#;
    assert!(messages(source)
        .iter()
        .any(|message| message.contains("not statically auditable")));
}
