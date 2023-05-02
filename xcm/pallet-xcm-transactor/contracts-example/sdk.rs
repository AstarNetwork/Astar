use core::marker::PhantomData;
use ink::env::{chain_extension::FromStatusCode, DefaultEnvironment, Environment};
use scale::{Decode, Encode};
use xcm::{latest::Weight, prelude::*};
use xcm_ce_primitives::{
    create_error_enum, Command, QueryConfig, ValidateSendInput, XCM_EXTENSION_ID,
};

create_error_enum!(pub Error);

impl FromStatusCode for Error {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            code => Err(code.into()),
        }
    }
}

/// XCM Chain Extension Interface
pub struct XcmExtension<E = DefaultEnvironment, const ID: u16 = XCM_EXTENSION_ID>(PhantomData<E>);

impl<E: Environment, const ID: u16> XcmExtension<E, ID> {
    const fn get_func_id(idx: u16) -> u32 {
        ((ID as u32) << 16) + (idx as u32)
    }

    pub fn prepare_execute(xcm: VersionedXcm<()>) -> Result<Weight, Error> {
        let func_id: u32 = Self::get_func_id(Command::PrepareExecute.into());

        // fn(VersionedXcm<()>) -> Result<Weight, Error>
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<VersionedXcm<()>>()
            .output::<Weight, false>()
            .handle_error_code::<Error>()
            .call(&(xcm))
    }

    pub fn execute() -> Result<(), Error> {
        let func_id: u32 = Self::get_func_id(Command::Execute.into());

        // fn() -> Result<(Weight), Error>
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<()>()
            .output::<(), false>()
            .handle_error_code::<Error>()
            .call(&())
    }

    pub fn validate_send(input: ValidateSendInput) -> Result<VersionedMultiAssets, Error> {
        let func_id: u32 = Self::get_func_id(Command::ValidateSend.into());

        // fn(ValidateSendInput) -> Result<VersionedMultiAssets, Error>
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<ValidateSendInput>()
            .output::<VersionedMultiAssets, false>()
            .handle_error_code::<Error>()
            .call(&(input))
    }

    pub fn send() -> Result<(), Error> {
        let func_id: u32 = Self::get_func_id(Command::Send.into());

        // fn() -> Result<(), Error>
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<()>()
            .output::<(), false>()
            .handle_error_code::<Error>()
            .call(&())
    }

    pub fn new_query(
        config: QueryConfig<E::AccountId, E::BlockNumber>,
        dest: VersionedMultiLocation,
    ) -> Result<QueryId, Error> {
        let func_id: u32 = Self::get_func_id(Command::NewQuery.into());

        // fn(QueryConfig, VersionedMultiLocation) -> Result<QueryId, Error>
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<(
                QueryConfig<E::AccountId, E::BlockNumber>,
                VersionedMultiLocation,
            )>()
            .output::<QueryId, false>()
            .handle_error_code::<Error>()
            .call(&(config, dest))
    }

    pub fn take_response(query_id: QueryId) -> Result<Response, Error> {
        let func_id: u32 = Self::get_func_id(Command::TakeResponse.into());

        // fn(QueryId) -> Result<Response, Error>
        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<QueryId>()
            .output::<Response, false>()
            .handle_error_code::<Error>()
            .call(&(query_id))
    }

    pub fn pallet_account_id() -> E::AccountId {
        let func_id = Self::get_func_id(Command::PalletAccountId.into());

        ::ink::env::chain_extension::ChainExtensionMethod::build(func_id)
            .input::<()>()
            .output::<E::AccountId, false>()
            .ignore_error_code()
            .call(&())
    }
}
