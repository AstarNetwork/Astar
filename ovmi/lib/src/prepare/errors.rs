use crate::compiled_predicates::*;
use snafu::{ResultExt, Snafu};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Happened Codec Error by: {}", err))]
    CodecError { err: codec::Error },
    #[snafu(display("Logic error by : {}", (*r#type).deserialize()))]
    LogicError { r#type: PredicateType },
}
