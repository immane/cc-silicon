use std::path::PathBuf;

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: cargo run --manifest-path tools/chip-lint/Cargo.toml -- <chip-source-directory>");
        std::process::exit(2);
    };

    match cc_silicon_chip_lint::lint_tree(&root) {
        Ok(diagnostics) if diagnostics.is_empty() => {
            println!("strict chip lint passed: {}", root.display());
        }
        Ok(diagnostics) => {
            for diagnostic in diagnostics {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("strict chip lint failed closed: {error}");
            std::process::exit(2);
        }
    }
}
