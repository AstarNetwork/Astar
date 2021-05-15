//! Plasm Parachain Collator. 

#![warn(missing_docs)]

fn main() -> Result<(), sc_cli::Error> {
    plasm_collator::run()
}
