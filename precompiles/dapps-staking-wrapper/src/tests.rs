use crate::mock::{
    advance_to_era, default_context, exit_error, initialize_first_block,
    precompile_address, ExternalityBuilder, Precompiles,
};
use crate::PrecompileOutput;
use pallet_evm::{ExitSucceed, PrecompileSet};
use sha3::{Digest, Keccak256};

#[test]
fn selector_out_of_bounds_nok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // Use 3 bytes selector. 4 bytes are needed
        let selector_nok = vec![0x01, 0x02, 0x03];

        let expected = Some(Err(exit_error("Selector too short")));

        assert_eq!(
            Precompiles::execute(
                precompile_address(),
                &selector_nok,
                None,
                &default_context(),
            ),
            expected
        );
    });
}
#[test]
fn selector_unknown_nok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // We use 3 bytes selector. 4 byts are needed
        let selector_nok = vec![0x01, 0x02, 0x03, 0x04];

        let expected = Some(Err(exit_error("No method at selector given selector")));

        assert_eq!(
            Precompiles::execute(
                precompile_address(),
                &selector_nok,
                None,
                &default_context(),
            ),
            expected
        );
    });
}

#[test]
fn current_era_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let selector = &Keccak256::digest(b"current_era()")[0..4];
        let mut expected_era = vec![0u8; 32];
        expected_era[31] = 1;

        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_era.clone(),
            cost: Default::default(),
            logs: Default::default(),
        }));

        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            expected
        );

        // advance to era 5 and check output
        expected_era[31] = 5;
        advance_to_era(5);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_era,
            cost: Default::default(),
            logs: Default::default(),
        }));
        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            expected
        );
    });
}
