// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

extern crate alloc;
use crate::{test::mock::*, *};

use assert_matches::assert_matches;

#[test]
fn smart_contract_types_are_ok() {
    // Verify Astar EVM smart contract type
    {
        let index: u8 = SmartContractTypes::Evm.into();
        assert_eq!(index, 0);
        assert_eq!(Ok(SmartContractTypes::Evm), index.try_into());
    }

    // Verify Astar WASM smart contract type
    {
        let index: u8 = SmartContractTypes::Wasm.into();
        assert_eq!(index, 1);
        assert_eq!(Ok(SmartContractTypes::Wasm), index.try_into());
    }

    // Negative case
    {
        let index: u8 = 2;
        let maybe_smart_contract: Result<SmartContractTypes, _> = index.try_into();
        assert_matches!(maybe_smart_contract, Err(_));
    }
}

#[test]
fn decode_smart_contract_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Astar EVM smart contract decoding
        {
            let address = H160::repeat_byte(0xCA);
            let smart_contract_v2 = SmartContractV2 {
                contract_type: SmartContractTypes::Evm,
                address: address.as_bytes().into(),
            };

            assert_eq!(
                Ok(<Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(address)),
                DappStakingV3Precompile::<Test>::decode_smart_contract(smart_contract_v2)
            );
        }

        // Astar WASM smart contract decoding
        {
            let address = [0x6E; 32];
            let smart_contract_v2 = SmartContractV2 {
                contract_type: SmartContractTypes::Wasm,
                address: address.into(),
            };

            assert_eq!(
                Ok(<Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(address.into())),
                DappStakingV3Precompile::<Test>::decode_smart_contract(smart_contract_v2)
            );
        }
    });
}

#[test]
fn decode_smart_contract_fails_when_type_and_address_mismatch() {
    ExternalityBuilder::build().execute_with(|| {
        // H160 address for Wasm smart contract type
        {
            let address = H160::repeat_byte(0xCA);
            let smart_contract_v2 = SmartContractV2 {
                contract_type: SmartContractTypes::Wasm,
                address: address.as_bytes().into(),
            };

            assert_matches!(
                DappStakingV3Precompile::<Test>::decode_smart_contract(smart_contract_v2),
                Err(_)
            );
        }

        // Native address for EVM smart contract type
        {
            let address = [0x6E; 32];
            let smart_contract_v2 = SmartContractV2 {
                contract_type: SmartContractTypes::Evm,
                address: address.into(),
            };

            assert_matches!(
                DappStakingV3Precompile::<Test>::decode_smart_contract(smart_contract_v2),
                Err(_)
            );
        }
    });
}

#[test]
fn parse_input_address_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // H160 address
        {
            let address_h160 = H160::repeat_byte(0xCA);
            let address_native = AddressMapper::into_account_id(address_h160);

            assert_eq!(
                DappStakingV3Precompile::<Test>::parse_input_address(
                    address_h160.as_bytes().into()
                ),
                Ok(address_native)
            );
        }

        // Native address
        {
            let address_native = [0x6E; 32];

            assert_eq!(
                DappStakingV3Precompile::<Test>::parse_input_address(address_native.into()),
                Ok(address_native.into())
            );
        }
    });
}

#[test]
fn parse_input_address_fails_with_incorrect_address_length() {
    ExternalityBuilder::build().execute_with(|| {
        let addresses: Vec<&[u8]> = vec![&[0x6E; 19], &[0xA1; 21], &[0xC3; 31], &[0x99; 33]];

        for address in addresses {
            assert_matches!(
                DappStakingV3Precompile::<Test>::parse_input_address(address.into()),
                Err(_)
            );
        }
    });
}
