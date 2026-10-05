use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprMacro, ExprMethodCall, File, ImplItem, Item, ItemImpl, Macro, Type};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

const DENIED_PATH_PREFIXES: &[&str] = &[
    "std::fs",
    "std::process",
    "std::env",
    "std::time",
    "std::thread",
    "std::net",
    "std::io",
    "std::sync",
    "rand",
    "getrandom",
];

const DENIED_IDENTIFIERS: &[&str] = &[
    "Instant",
    "SystemTime",
    "Command",
    "File",
    "OpenOptions",
    "TcpStream",
    "UdpSocket",
];

/// Lint a source string expected to contain chip implementations.
pub fn lint_source(file: &str, source: &str) -> Vec<Diagnostic> {
    let parsed = match syn::parse_file(source) {
        Ok(parsed) => parsed,
        Err(error) => {
            return vec![Diagnostic {
                file: file.to_owned(),
                line: error.span().start().line,
                message: format!(
                    "Rust syntax could not be analyzed; refusing strict lint: {error}"
                ),
            }];
        }
    };

    lint_file(file, &parsed)
}

/// Lint every Rust file below a chip source directory.
pub fn lint_tree(root: &Path) -> Result<Vec<Diagnostic>, String> {
    if !root.is_dir() {
        return Err(format!(
            "chip source directory does not exist: {}",
            root.display()
        ));
    }

    let mut files = Vec::new();
    collect_rust_files(root, &mut files)?;
    files.sort();

    if files.is_empty() {
        return Err(format!("no Rust files found under {}", root.display()));
    }

    let mut diagnostics = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        diagnostics.extend(lint_source(&relative, &source));
    }
    Ok(diagnostics)
}

fn collect_rust_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|error| format!("cannot list {}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("directory entry error: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
    Ok(())
}

fn lint_file(file: &str, syntax: &File) -> Vec<Diagnostic> {
    let mut known_chip_names = HashSet::new();
    let mut macro_impls = Vec::new();
    let known_functions = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) => Some(function.sig.ident.to_string()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    for item in &syntax.items {
        match item {
            Item::Struct(item) if item.ident.to_string().ends_with("Chip") => {
                known_chip_names.insert(item.ident.to_string());
                if !matches!(item.fields, syn::Fields::Unit) {
                    return vec![diagnostic(file, item, "chip structs must be unit structs")];
                }
            }
            Item::Macro(item) if macro_name(&item.mac) == "silicon_chip" => {
                match parse_chip_macro(&item.mac) {
                    Ok((chip_name, implementation)) => {
                        known_chip_names.insert(chip_name);
                        macro_impls.push(implementation);
                    }
                    Err(message) => {
                        return vec![diagnostic(file, item, &message)];
                    }
                }
            }
            _ => {}
        }
    }

    let mut diagnostics = Vec::new();
    for implementation in &macro_impls {
        check_chip_impl(
            file,
            implementation,
            &known_chip_names,
            &known_functions,
            &mut diagnostics,
        );
    }
    for item in &syntax.items {
        match item {
            Item::Impl(item_impl) if is_chip_impl(item_impl) => {
                check_chip_impl(
                    file,
                    item_impl,
                    &known_chip_names,
                    &known_functions,
                    &mut diagnostics,
                );
            }
            Item::Impl(item_impl) if is_worker_compute_impl(item_impl) => {
                check_worker_compute_impl(
                    file,
                    item_impl,
                    &known_chip_names,
                    &known_functions,
                    &mut diagnostics,
                );
            }
            Item::Impl(item_impl) if is_legacy_logic_chip_impl(item_impl) => {
                diagnostics.push(diagnostic(
                    file,
                    item_impl,
                    "legacy LogicChip receives the full mutable bus; use RestrictedChip",
                ));
            }
            Item::Use(item_use) => {
                let path = item_use.tree.to_token_stream().to_string().replace(' ', "");
                let root = path.split(&[':', '{'][..]).next().unwrap_or_default();
                // `/9` PCR-10: deterministic BTreeMap/BTreeSet and sibling
                // chip-module re-exports are mechanical, not Host I/O.
                let allowed_std_collections = path.starts_with("std::collections::")
                    && (path.contains("BTreeMap") || path.contains("BTreeSet"));
                let allowed_root = matches!(
                    root,
                    "crate"
                        | "self"
                        | "super"
                        | "core"
                        | "alloc"
                        | "fold"
                        | "pp_normalize"
                        | "lx_intern"
                        | "lx_classify"
                        | "lx_decode"
                ) || allowed_std_collections;
                if is_denied_path(&path) || !allowed_root {
                    diagnostics.push(diagnostic(
                        file,
                        item_use,
                        "Host or nondeterministic API import in chip source",
                    ));
                }
            }
            Item::Macro(item_macro) if macro_name(&item_macro.mac) != "silicon_chip" => {
                diagnostics.push(diagnostic(
                    file,
                    item_macro,
                    "unexpanded top-level macro is not permitted in strict chip source",
                ));
            }
            _ => {}
        }
    }
    diagnostics
}

fn parse_chip_macro(mac: &Macro) -> Result<(String, ItemImpl), String> {
    struct ChipMacroItems {
        name: String,
        implementation: ItemImpl,
    }

    impl syn::parse::Parse for ChipMacroItems {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let _attributes = input.call(syn::Attribute::parse_outer)?;
            let _visibility: syn::Visibility = input.parse()?;
            input.parse::<syn::Token![struct]>()?;
            let name: syn::Ident = input.parse()?;
            input.parse::<syn::Token![;]>()?;
            let implementation: ItemImpl = input.parse()?;
            if !input.is_empty() {
                return Err(input.error("unexpected tokens after restricted chip implementation"));
            }
            Ok(Self {
                name: name.to_string(),
                implementation,
            })
        }
    }

    let parsed: ChipMacroItems = syn::parse2(mac.tokens.clone())
        .map_err(|error| format!("silicon_chip! declaration could not be linted: {error}"))?;
    if !is_chip_impl(&parsed.implementation) {
        return Err("silicon_chip! must implement RestrictedChip".to_owned());
    }
    if self_type_name(&parsed.implementation) != Some(parsed.name.clone()) {
        return Err("silicon_chip! implementation must target its declared unit struct".to_owned());
    }
    Ok((parsed.name, parsed.implementation))
}

fn check_chip_impl(
    file: &str,
    item_impl: &ItemImpl,
    known_chip_names: &HashSet<String>,
    known_functions: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_chip_impl(item_impl) {
        diagnostics.push(diagnostic(
            file,
            item_impl,
            "chip macro must implement RestrictedChip",
        ));
        return;
    }

    let Some(compute) = item_impl.items.iter().find_map(|item| match item {
        ImplItem::Fn(method) if method.sig.ident == "compute" => Some(method),
        _ => None,
    }) else {
        diagnostics.push(diagnostic(
            file,
            item_impl,
            "RestrictedChip implementation has no compute method",
        ));
        return;
    };

    let mut visitor = ChipBodyVisitor {
        file,
        known_chip_names,
        known_functions,
        diagnostics: Vec::new(),
    };
    visitor.visit_block(&compute.block);
    diagnostics.extend(visitor.diagnostics);
}

struct ChipBodyVisitor<'a> {
    file: &'a str,
    known_chip_names: &'a HashSet<String>,
    known_functions: &'a HashSet<String>,
    diagnostics: Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for ChipBodyVisitor<'_> {
    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        // `/9` PCR-10: `vec!`/`format!` are transparent deterministic
        // constructors, not opaque DSLs. All other macros stay opaque.
        if let Some(segment) = node.mac.path.segments.last() {
            let name = segment.ident.to_string();
            if name == "vec" || name == "format" {
                visit::visit_expr_macro(self, node);
                return;
            }
        }
        self.diagnostics.push(diagnostic(
            self.file,
            node,
            "macros inside compute are opaque to the strict lint; expand or explicitly audit them",
        ));
        visit::visit_expr_macro(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        let resolved = match node.func.as_ref() {
            Expr::Path(path) => match path.path.get_ident() {
                Some(ident) => {
                    let name = ident.to_string();
                    // UpperCamelCase single-ident calls are tuple-struct or
                    // enum-variant constructors (`DraftRef(0)`, `Some(x)`),
                    // deterministic by construction. Lowercase calls are
                    // function calls and must resolve to a same-file helper.
                    name.starts_with(|ch: char| ch.is_uppercase())
                        || self.known_functions.contains(&name)
                        || matches!(name.as_str(), "Some" | "Ok" | "Err")
                }
                None => true,
            },
            _ => false,
        };
        if !resolved {
            self.diagnostics.push(diagnostic(
                self.file,
                node,
                "indirect or unresolved function calls inside compute are not statically auditable",
            ));
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        if matches!(method.as_str(), "tick" | "clock_tick" | "execute_layers") {
            self.diagnostics.push(diagnostic(
                self.file,
                node,
                "chip-to-chip or motherboard execution calls are forbidden inside compute",
            ));
        }
        if let Expr::Path(path) = node.receiver.as_ref() {
            if let Some(name) = path.path.get_ident() {
                if self.known_chip_names.contains(&name.to_string()) {
                    self.diagnostics.push(diagnostic(
                        self.file,
                        node,
                        "calling another chip from compute is forbidden",
                    ));
                }
            }
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        let path = node
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if is_denied_path(&path)
            || node
                .path
                .segments
                .iter()
                .any(|segment| DENIED_IDENTIFIERS.contains(&segment.ident.to_string().as_str()))
        {
            self.diagnostics.push(diagnostic(
                self.file,
                node,
                "Host or nondeterministic API reference in chip computation",
            ));
        }
        visit::visit_expr_path(self, node);
    }

    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        if let Some(segment) = node.mac.path.segments.last() {
            let name = segment.ident.to_string();
            if name == "vec" || name == "format" {
                visit::visit_item_macro(self, node);
                return;
            }
        }
        self.diagnostics.push(diagnostic(
            self.file,
            node,
            "macros inside compute are opaque to the strict lint; expand or explicitly audit them",
        ));
        visit::visit_item_macro(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        if let Expr::Unsafe(unsafe_block) = node {
            self.diagnostics.push(diagnostic(
                self.file,
                unsafe_block,
                "unsafe blocks are forbidden in chip computation",
            ));
        }
        visit::visit_expr(self, node);
    }
}

fn is_chip_impl(item_impl: &ItemImpl) -> bool {
    item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
        path.segments
            .last()
            .is_some_and(|segment| segment.ident == "RestrictedChip")
    })
}

/// `/9` PCR-10: an inherent `impl XChip { fn compute(..) }` is the Worker
/// template's semantic body. It must be linted like `RestrictedChip::compute`,
/// otherwise Host I/O can hide behind the Worker adapter boundary.
fn is_worker_compute_impl(item_impl: &ItemImpl) -> bool {
    if item_impl.trait_.is_some() {
        return false;
    }
    let Some(name) = self_type_name(item_impl) else {
        return false;
    };
    if !name.ends_with("Chip") {
        return false;
    }
    item_impl.items.iter().any(|item| match item {
        ImplItem::Fn(method) => method.sig.ident == "compute",
        _ => false,
    })
}

fn check_worker_compute_impl(
    file: &str,
    item_impl: &ItemImpl,
    known_chip_names: &HashSet<String>,
    known_functions: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(compute) = item_impl.items.iter().find_map(|item| match item {
        ImplItem::Fn(method) if method.sig.ident == "compute" => Some(method),
        _ => None,
    }) else {
        return;
    };
    let mut visitor = ChipBodyVisitor {
        file,
        known_chip_names,
        known_functions,
        diagnostics: Vec::new(),
    };
    visitor.visit_block(&compute.block);
    diagnostics.extend(visitor.diagnostics);
}

fn is_legacy_logic_chip_impl(item_impl: &ItemImpl) -> bool {
    item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
        path.segments
            .last()
            .is_some_and(|segment| segment.ident == "LogicChip")
    })
}

fn self_type_name(item_impl: &ItemImpl) -> Option<String> {
    if let Type::Path(path) = item_impl.self_ty.as_ref() {
        path.path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
    } else {
        None
    }
}

fn macro_name(mac: &Macro) -> String {
    mac.path
        .segments
        .last()
        .map_or_else(String::new, |segment| segment.ident.to_string())
}

fn is_denied_path(path: &str) -> bool {
    DENIED_PATH_PREFIXES
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}::")))
}

fn diagnostic<T: syn::spanned::Spanned>(file: &str, node: &T, message: &str) -> Diagnostic {
    Diagnostic {
        file: file.to_owned(),
        line: node.span().start().line,
        message: message.to_owned(),
    }
}
