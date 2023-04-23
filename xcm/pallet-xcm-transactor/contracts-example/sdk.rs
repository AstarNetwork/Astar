use core::marker::PhantomData;
use ink::env::Environment;
use xcm::{latest::Weight, prelude::*};
use xcm_ce_types::Error;

/// XCM Chain Extension Interface
pub struct XcmExtension<E: Environment, const ID: u16 = 10>(PhantomData<E>);

impl<E: Environment, const ID: u16> XcmExtension<E, ID> {
    const fn get_func_id(idx: u16) -> u32 {
        ((ID as u32) << 16) + (idx as u32)
    }

    pub fn prepare_execute(xcm: VersionedXcm<()>) -> Result<Weight, Error> {
        let func_id: u32 = Self::get_func_id(0);
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<VersionedXcm<()>>()
            .output::<Weight, false>()
            .handle_error_code::<Error>()
            .call(&(xcm))
    }
}
