use crate::compiled_predicates::*;
use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("Happened Codec Error by: {}", err))]
    CodecError { err: codec::Error },
    #[snafu(display("Logic error by : {}", r#type))]
    LogicError { r#type: PredicateType },
}
