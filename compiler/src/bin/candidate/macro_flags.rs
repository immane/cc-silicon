// ============================================================================
// bin/candidate/macro_flags.rs — H04 `-D`/`-U` flag parsing helper.
//
// Pure host-side parsing only: splits argv `-D NAME[=value]` / `-U NAME`
// occurrences into validated seeds and builds the matching `MacroRecord`
// shapes once the integrator has scanned the value text. It performs no
// I/O, appends no bus records, and runs no chips; scanning the value
// bytes into pp-tokens and committing the records stays with the
// integrator (chips own every semantic decision).
//
// Seed semantics (mirror the frozen workers):
// - `-D NAME` defines `NAME` as `1` (GCC-compatible bare-`-D` default);
//   `-D NAME=value` keeps `value` verbatim (possibly empty: defined as
//   empty, which is distinct from `-U`).
// - `-U NAME` yields the PP undef tombstone (`undefined: true`), which
//   supersedes by ID order like any later definition.
// - Order is significant: the integrator must commit seeds in flag order;
//   lookup takes the greatest `MacroId` with equal spelling, so the last
//   `-D`/`-U` for a name wins without any dedup here.
//
// This file is inert until the integrator declares
// `#[path = "candidate/macro_flags.rs"] mod macro_flags;` in
// `candidate.rs` (a plain `mod` would resolve to `src/bin/`, where the
// file would become its own binary target) and performs the co-owned
// wiring (`-D`/`-U` arms in `parse_args`, value scanning, record
// commit); see the H04 report.
// ============================================================================

use cc_silicon_compiler::bus::MacroRecord;
use cc_silicon_compiler::ids::PpTokenId;

/// Default replacement text for a bare `-D NAME` (no `=` present).
pub const BARE_DEFINE_VALUE: &[u8] = b"1";

/// Parsed `-D NAME[=value]`: the validated name plus the raw value bytes
/// (verbatim text after the first `=`, or `BARE_DEFINE_VALUE`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefineSeed {
    /// Macro name spelling (validated C identifier bytes).
    pub name: Vec<u8>,
    /// Replacement text bytes (may be empty for `-D NAME=`).
    pub value: Vec<u8>,
}

/// Parsed `-U NAME`: the validated name to undefine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndefSeed {
    /// Macro name spelling (validated C identifier bytes).
    pub name: Vec<u8>,
}

/// One parsed macro flag, in argv order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MacroFlagSeed {
    /// `-D NAME[=value]`.
    Define(DefineSeed),
    /// `-U NAME`.
    Undef(UndefSeed),
}

/// Split one argv element into its flag kind and body: `Some(('D', body))`
/// for `-D<body>`, `Some(('U', body))` for `-U<body>`, else `None`. An
/// empty body means the separate-argument form (`-D NAME`): the caller
/// takes the next argv element as the body.
pub fn split_flag(arg: &str) -> Option<(char, &str)> {
    if let Some(body) = arg.strip_prefix("-D") {
        return Some(('D', body));
    }
    if let Some(body) = arg.strip_prefix("-U") {
        return Some(('U', body));
    }
    None
}

/// Parse a `-D` body (`NAME` or `NAME=value`) into a seed. The value is
/// everything after the first `=` kept verbatim; scanning it into
/// pp-tokens is the integrator's job.
pub fn parse_define_body(body: &str) -> Result<DefineSeed, String> {
    let bytes = body.as_bytes();
    let (name, value) = match bytes.iter().position(|byte| *byte == b'=') {
        Some(index) => (&bytes[..index], bytes[index + 1..].to_vec()),
        None => (bytes, BARE_DEFINE_VALUE.to_vec()),
    };
    if name.is_empty() {
        return Err(format!("-D requires a macro name (got {body:?})"));
    }
    if !is_valid_name(name) {
        return Err(format!(
            "-D has an invalid macro name {:?}: expected a C identifier",
            String::from_utf8_lossy(name)
        ));
    }
    Ok(DefineSeed {
        name: name.to_vec(),
        value,
    })
}

/// Parse a `-U` body (`NAME`) into a seed. A body containing `=` is
/// rejected by the identifier check (`=` is never part of a name).
pub fn parse_undef_body(body: &str) -> Result<UndefSeed, String> {
    if body.is_empty() {
        return Err("-U requires a macro name".to_string());
    }
    if !is_valid_name(body.as_bytes()) {
        return Err(format!(
            "-U has an invalid macro name {body:?}: expected a C identifier"
        ));
    }
    Ok(UndefSeed {
        name: body.as_bytes().to_vec(),
    })
}

/// Build the `-U` tombstone record for a validated name. Mirrors the
/// frozen PP undef worker: object-like shape with `undefined: true` and
/// no replacement.
pub fn undef_record(name: Vec<u8>) -> MacroRecord {
    MacroRecord {
        spelling: name,
        params: Vec::new(),
        function_like: false,
        variadic: false,
        replacement: Vec::new(),
        undefined: true,
    }
}

/// Build the `-D` definition record for a validated name once the value
/// text has been scanned into pp-tokens. Mirrors the frozen PP06 fresh
/// path: object-like, non-variadic, `undefined: false`.
pub fn define_record(name: Vec<u8>, replacement: Vec<PpTokenId>) -> MacroRecord {
    MacroRecord {
        spelling: name,
        params: Vec::new(),
        function_like: false,
        variadic: false,
        replacement,
        undefined: false,
    }
}

/// True when `bytes` is a C identifier (`[A-Za-z_][A-Za-z0-9_]*`, ASCII
/// only; command-line names never carry another encoding).
fn is_valid_name(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let head = bytes[0];
    if !(head.is_ascii_alphabetic() || head == b'_') {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_define_defaults_to_one() {
        assert_eq!(
            parse_define_body("FOO").expect("bare name parses"),
            DefineSeed {
                name: b"FOO".to_vec(),
                value: b"1".to_vec(),
            }
        );
    }

    #[test]
    fn define_keeps_value_verbatim() {
        assert_eq!(
            parse_define_body("FOO=bar(baz) 42").expect("valued define parses"),
            DefineSeed {
                name: b"FOO".to_vec(),
                value: b"bar(baz) 42".to_vec(),
            }
        );
    }

    #[test]
    fn define_splits_on_first_equals() {
        assert_eq!(
            parse_define_body("FOO=a=b").expect("second `=` stays in the value"),
            DefineSeed {
                name: b"FOO".to_vec(),
                value: b"a=b".to_vec(),
            }
        );
    }

    #[test]
    fn define_empty_value_is_defined_empty() {
        assert_eq!(
            parse_define_body("FOO=").expect("empty value parses"),
            DefineSeed {
                name: b"FOO".to_vec(),
                value: Vec::new(),
            }
        );
    }

    #[test]
    fn define_rejects_missing_and_bad_names() {
        assert!(parse_define_body("").is_err());
        assert!(parse_define_body("=1").is_err());
        assert!(parse_define_body("9LIVES=1").is_err());
        assert!(parse_define_body("FOO-BAR=1").is_err());
        assert!(parse_define_body("F O O").is_err());
    }

    #[test]
    fn undef_round_trip() {
        assert_eq!(
            parse_undef_body("FOO").expect("name parses"),
            UndefSeed {
                name: b"FOO".to_vec(),
            }
        );
        assert!(parse_undef_body("").is_err());
        assert!(parse_undef_body("FOO=1").is_err());
        assert!(parse_undef_body("9LIVES").is_err());
    }

    #[test]
    fn split_flag_forms() {
        assert_eq!(split_flag("-D"), Some(('D', "")));
        assert_eq!(split_flag("-U"), Some(('U', "")));
        assert_eq!(split_flag("-DFOO=1"), Some(('D', "FOO=1")));
        assert_eq!(split_flag("-UFOO"), Some(('U', "FOO")));
        assert_eq!(split_flag("-O0"), None);
        assert_eq!(split_flag("main.c"), None);
    }

    #[test]
    fn flag_seed_ordering_shape() {
        let flags = [
            MacroFlagSeed::Define(parse_define_body("FOO=1").expect("define parses")),
            MacroFlagSeed::Undef(parse_undef_body("FOO").expect("undef parses")),
        ];
        assert!(matches!(flags[0], MacroFlagSeed::Define(_)));
        assert!(matches!(flags[1], MacroFlagSeed::Undef(_)));
    }

    #[test]
    fn records_mirror_frozen_shapes() {
        let defined = define_record(b"FOO".to_vec(), Vec::new());
        assert!(!defined.undefined);
        assert!(!defined.function_like);
        assert!(!defined.variadic);
        assert!(defined.params.is_empty());
        let tombstone = undef_record(b"FOO".to_vec());
        assert!(tombstone.undefined);
        assert!(tombstone.replacement.is_empty());
    }
}
