//! Plasm Node executable.

#![warn(missing_docs)]

fn main() -> Result<(), plasm_cli::Error> {
    plasm_cli::run()
}
