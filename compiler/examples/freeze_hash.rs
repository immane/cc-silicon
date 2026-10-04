//! Print the frozen contract content hash.
//!
//! Run with `cargo run --manifest-path compiler/Cargo.toml --example freeze_hash`
//! after any change to the frozen schema, then update `CONTRACT_HASH` and
//! `contracts/CONTRACT_VERSION`.

fn main() {
    println!("{}", cc_silicon_compiler::compute_contract_hash());
}
