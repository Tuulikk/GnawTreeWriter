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
    // clap's `debug_assert` walks the WHOLE table (≈100 subcommands with long
    // about-texts), and a Rust test thread gets 2 MiB of stack, not the 8 MiB
    // the main thread has. Adding the `head` variant pushed it over and the
    // process aborted with SIGABRT instead of reporting — measured 2026-11-12.
    // The contract is about the TABLE, not about fitting in the harness stack,
    // so the assertion runs on a thread with room.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            Cli::command().debug_assert();
        })
        .expect("spawn assertion thread")
        .join()
        .expect("the command table must be valid");
}
