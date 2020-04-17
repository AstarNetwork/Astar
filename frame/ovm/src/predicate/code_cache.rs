use super::super::*;
use crate::predicate::*;

/// Put code in the storage. The hash of code is used as a key and is returned
/// as a result of this function.
///
/// This function instruments the given code and caches it in the storage.
pub fn save<T: Trait>(
    original_code: Vec<u8>,
    schedule: &Schedule,
) -> Result<PredicateHash<T>, &'static str> {
    let prefab = prepare::prepare_predicate(&original_code, schedule)?;
    let predicate_hash = <T as system::Trait>::Hashing::hash_of(&original_code);
    <PredicateCodes<T>>::insert(&predicate_hash, original_code);
    <PredicateCache<T>>::insert(&predicate_hash, prefab);

    Ok(predicate_hash)
}

/// Load code with the given code hash.
///
/// If the module was instrumented with a lower version of schedule than
/// the current one given as an argument, then this function will perform
/// re-instrumentation and update the cache in the storage.
pub fn load<T: Trait>(
    code_hash: &PredicateHash<T>,
    schedule: &Schedule,
) -> Result<PrefabOvmModule, &'static str> {
    let mut prefab_module =
        <PredicateCache<T>>::get(code_hash).ok_or_else(|| "code is not found")?;

    if prefab_module.schedule_version < schedule.version {
        // The current schedule version is greater than the version of the one cached
        // in the storage.
        //
        // We need to re-instrument the code with the latest schedule here.
        let original_code =
            <PredicateCodes<T>>::get(code_hash).ok_or_else(|| "predicate code is not found")?;
        prefab_module = prepare::prepare_predicate(&original_code, schedule)?;
        <PredicateCache<T>>::insert(&code_hash, &prefab_module);
    }
    Ok(prefab_module)
}
