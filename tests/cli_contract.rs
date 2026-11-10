//! CLI contract: the argument table must be structurally valid.
//!
//! clap validates an argument table at *runtime* under `debug_assertions` and
//! panics on violations. Release builds compile that check out, so a broken
//! table "works" there while behaving ambiguously (a duplicate `-m` silently
//! shadows one flag; a required positional after an optional one misassigns
//! arguments when the optional is omitted).
//!
//! Four subcommands shipped in exactly that state and were found only by
//! invoking each one by hand — `insert` panicked on EVERY call in a debug
//! build (`parent_path` index 2), `curate` had two `-m`, `pack` had two `-i`,
//! and `restore` had a `bool` positional. This test makes that class impossible
//! to reintroduce, without needing a fixture or a running binary.
use clap::CommandFactory;
use gnawtreewriter::cli::Cli;

#[test]
fn cli_command_table_is_well_formed() {
    Cli::command().debug_assert();
}
