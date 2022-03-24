//! Astar XCM tools binary.

#![warn(missing_docs)]

mod cli;
mod command;

fn main() -> Result<(), command::Error> {
    command::run()
}
