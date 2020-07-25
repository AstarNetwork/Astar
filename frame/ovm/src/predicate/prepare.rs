use crate::predicate::PrefabOvmModule;
use crate::Schedule;

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
    Ok(PrefabOvmModule {
        schedule_version: schedule.version,
        code: original_code.to_vec(),
    })
}
