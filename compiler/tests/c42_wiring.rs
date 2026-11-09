// ============================================================================
// c42_wiring.rs — chip-tree wiring and reachability guard (WO-A13-03 / B1-06).
//
// Every `.rs` file under `compiler/src/chips` other than a `mod.rs` must be
// declared by a `mod <name>;` in its parent `mod.rs`. The nine user-owned
// drafts that are intentionally not built are listed in `DRAFT_FILES`; the
// list must stay exact, so a draft that becomes wired fails with an explicit
// instruction to remove its parked entry.
//
// The guard also checks the reverse direction: every `mod <name>;` declaration
// in the tree must resolve to an existing source file (`<name>.rs` or
// `<name>/mod.rs`). No dependency is used; `std::fs` is sufficient.
// ============================================================================

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const CHIPS_REL: &str = "src/chips";

/// Unwired user drafts, relative to `compiler/src/chips`.
///
/// This list is deliberately exact: each entry is a user-owned draft awaiting
/// integrator disposition. Remove an entry when the module is wired; the guard
/// fails if an entry is stale or if a wired module keeps its entry.
const DRAFT_FILES: &[&str] = &[
    "lex/lx_error.rs",
    "lex/lx_extension.rs",
    "parse/pa_cast.rs",
    "parse/pa_decl_spec.rs",
    "parse/pa_expr_ext.rs",
    "parse/pa_ext_dispatch.rs",
    "parse/pa_init.rs",
    "parse/pa_stmt.rs",
    "parse/pa_typedef_query.rs",
];

#[test]
fn chip_sources_are_wired_or_explicitly_parked() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join(CHIPS_REL);

    let mut sources = Vec::new();
    collect_rust_files(&root, &mut sources);
    sources.sort();

    let mut declarations: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    for path in &sources {
        if path.file_name() == Some(OsStr::new("mod.rs")) {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", chip_rel(path, &root)));
            declarations.insert(path.clone(), mod_declarations(&source));
        }
    }

    let mut errors = Vec::new();
    let drafts: BTreeSet<&str> = DRAFT_FILES.iter().copied().collect();
    if drafts.len() != DRAFT_FILES.len() {
        errors.push("DRAFT_FILES contains duplicate entries".to_string());
    }

    for draft in DRAFT_FILES {
        if !root.join(draft).is_file() {
            errors.push(format!(
                "DRAFT_FILES entry {draft} does not exist; remove it from compiler/tests/c42_wiring.rs"
            ));
        }
    }

    let mut orphans = Vec::new();
    for path in &sources {
        if path.file_name() == Some(OsStr::new("mod.rs")) {
            continue;
        }
        let relative = chip_rel(path, &root);
        let Some(parent) = path.parent() else {
            errors.push(format!("{relative} has no parent directory"));
            continue;
        };
        let Some(stem) = path.file_stem().and_then(OsStr::to_str) else {
            errors.push(format!("{relative} has a non-UTF-8 file stem"));
            continue;
        };
        let wired = declarations
            .get(&parent.join("mod.rs"))
            .is_some_and(|names| names.contains(stem));
        if wired {
            if drafts.contains(relative.as_str()) {
                errors.push(format!(
                    "{relative} is now declared by {}/mod.rs; remove it from DRAFT_FILES in compiler/tests/c42_wiring.rs",
                    chip_rel(parent, &root)
                ));
            }
        } else if drafts.contains(relative.as_str()) {
            // Explicitly parked user draft: intentionally not built.
        } else {
            orphans.push(format!(
                "{relative} is not declared by {}/mod.rs",
                chip_rel(parent, &root)
            ));
        }
    }
    if !orphans.is_empty() {
        errors.push(format!(
            "unwired chip source file(s):\n    {}\n  wire each file with `mod <name>;` in its parent mod.rs, or add it to DRAFT_FILES in compiler/tests/c42_wiring.rs with integrator review",
            orphans.join("\n    ")
        ));
    }

    for (mod_rs, names) in &declarations {
        let Some(parent) = mod_rs.parent() else {
            errors.push(format!(
                "{} has no parent directory",
                chip_rel(mod_rs, &root)
            ));
            continue;
        };
        for name in names {
            let file = parent.join(format!("{name}.rs"));
            let nested = parent.join(name).join("mod.rs");
            if !file.is_file() && !nested.is_file() {
                errors.push(format!(
                    "{} declares `mod {name};`, but neither {} nor {} exists",
                    chip_rel(mod_rs, &root),
                    chip_rel(&file, &root),
                    chip_rel(&nested, &root)
                ));
            }
        }
    }

    assert!(
        errors.is_empty(),
        "chip wiring guard failed:\n- {}",
        errors.join("\n- ")
    );
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("cannot read an entry in {}: {error}", directory.display())
        });
        paths.push(entry.path());
    }
    paths.sort();
    for path in paths {
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|error| panic!("cannot read metadata for {}: {error}", path.display()));
        if metadata.is_dir() {
            collect_rust_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == OsStr::new("rs"))
        {
            files.push(path);
        }
    }
}

/// Path relative to the chip tree, normalized to `/` for stable messages.
fn chip_rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Collect `mod <name>;` declarations, skipping comments and string literals.
///
/// Inline modules (`mod tests { ... }`) have no semicolon and are ignored, as
/// are `mod` words inside comments or strings.
fn mod_declarations(source: &str) -> BTreeSet<String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = skip_block_comment(bytes, index);
            }
            b'"' => {
                index = skip_string(bytes, index);
            }
            b'r' if is_raw_string_start(bytes, index) => {
                index = skip_raw_string(bytes, index);
            }
            byte if is_ident_start(byte) => {
                let start = index;
                index += 1;
                while index < bytes.len() && is_ident_continue(bytes[index]) {
                    index += 1;
                }
                tokens.push(&source[start..index]);
            }
            b';' => {
                tokens.push(";");
                index += 1;
            }
            _ => index += 1,
        }
    }

    let mut declarations = BTreeSet::new();
    for window in tokens.windows(3) {
        if window[0] == "mod" && window[1] != ";" && window[2] == ";" {
            declarations.insert(window[1].to_string());
        }
    }
    declarations
}

fn is_ident_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_ident_continue(byte: u8) -> bool {
    is_ident_start(byte) || byte.is_ascii_digit()
}

fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 2;
    let mut depth = 1;
    while index < bytes.len() && depth > 0 {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            depth += 1;
            index += 2;
        } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
            depth -= 1;
            index += 2;
        } else {
            index += 1;
        }
    }
    index
}

fn skip_string(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    index
}

fn is_raw_string_start(bytes: &[u8], start: usize) -> bool {
    if bytes.get(start) != Some(&b'r') {
        return false;
    }
    let mut index = start + 1;
    while bytes.get(index) == Some(&b'#') {
        index += 1;
    }
    bytes.get(index) == Some(&b'"')
}

fn skip_raw_string(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 1;
    let mut hashes = 0;
    while bytes.get(index) == Some(&b'#') {
        hashes += 1;
        index += 1;
    }
    // `index` is at the opening quote.
    index += 1;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            let mut check = index + 1;
            let mut seen = 0;
            while seen < hashes && bytes.get(check) == Some(&b'#') {
                seen += 1;
                check += 1;
            }
            if seen == hashes {
                return check;
            }
        }
        index += 1;
    }
    index
}
