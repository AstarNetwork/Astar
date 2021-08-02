//! Astar collator binary.

#![warn(missing_docs)]

fn main() -> Result<(), sc_cli::Error> {
    astar_collator::run()
}
