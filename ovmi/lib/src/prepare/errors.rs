use crate::compiled_predicates::*;
use snafu::{ResultExt, Snafu};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Snafu)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Error {
    #[snafu(display("Happened Codec Error by: {}", err))]
    CodecError { err: codec::Error },
    #[snafu(display("Logic error by : {}", r#type))]
    LogicError { r#type: PredicateType },
}
