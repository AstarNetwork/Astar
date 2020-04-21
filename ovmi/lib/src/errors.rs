use crate::compiled_predicate::*;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum ExecError {
    #[snafu(display("Require error: {}", msg))]
    RequireError { msg: String },
    #[snafu(display("Unexpected error: {}", msg))]
    UnexpectedError { msg: String },
}
