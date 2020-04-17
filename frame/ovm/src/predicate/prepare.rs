use crate::predicate::PrefabOvmModule;
use crate::Schedule;

struct PredicateModule<'a> {
    /// A deserialized module. The predicate module is valid (this is Guaranteed by `new` method).
    // module: elements::Module,
    schedule: &'a Schedule,
}

impl<'a> PredicateModule<'a> {
    /// Creates a new instance of `PredicateModule`.
    ///
    /// Returns `Err` if the `original_code` couldn't be decoded or
    /// if it contains an invalid module.
    fn new(original_code: &[u8], schedule: &'a Schedule) -> Result<Self, &'static str> {
        // TODO use ovm validations.
        // use wasmi_validation::{validate_module, PlainValidator};

        // TODO deserialize(otimized in rust) ovm buffer.
        // let module =
        //     elements::deserialize_buffer(original_code).map_err(|_| "Can't decode wasm code")?;

        // TODO validate
        // Make sure that the module is valid.
        // validate_module::<PlainValidator>(&module).map_err(|_| "Module is not valid")?;

        // TODO
        // Return a `PredicateModule` instance with
        // __valid__ module.
        Ok(PredicateModule {
            // module,
            schedule,
        })
    }

    fn into_ovm_code(self) -> Result<Vec<u8>, &'static str> {
        // TODO into self to ovm srialized codes.
        //        elements::serialize(self.module)
        //             .map_err(|_| "error serializing instrumented module")
        Ok(vec![])
    }
}

/// Loads the given module given in `original_code`, performs some checks on it and
/// does some preprocessing.
///
/// The checks are:
///
/// - provided code is a valid predicate codes in ovm.
///
/// The preprocessing includes injecting code for metering the height of stack.
pub fn prepare_predicate(
    original_code: &[u8],
    schedule: &Schedule,
) -> Result<PrefabOvmModule, &'static str> {
    let mut predicate_module = PredicateModule::new(original_code, schedule)?;
    Ok(PrefabOvmModule {
        schedule_version: schedule.version,
        code: predicate_module.into_ovm_code()?,
    })
}
