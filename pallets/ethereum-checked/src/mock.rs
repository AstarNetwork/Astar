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

#![cfg(test)]

use super::*;
use crate as pallet_ethereum_checked;

use ethereum_types::H256;

use frame_support::{
    assert_ok, construct_runtime, parameter_types,
    sp_io::TestExternalities,
    traits::{ConstU128, ConstU64, FindAuthor},
    weights::Weight,
};
use pallet_ethereum::PostLogContent;
use pallet_evm::{AddressMapping, FeeCalculator};
use sp_io::hashing::blake2_256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, ConsensusEngineId,
};

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type RuntimeCall = RuntimeCall;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<2>;
    type AccountStore = System;
    type WeightInfo = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<3>;
    type WeightInfo = ();
}

pub struct MockFeeCalculator;
impl FeeCalculator for MockFeeCalculator {
    fn min_gas_price() -> (U256, Weight) {
        (U256::one(), Weight::zero())
    }
}

pub struct MockFindAuthor;
impl FindAuthor<H160> for MockFindAuthor {
    fn find_author<'a, I>(_digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        Some(H160::from_low_u64_be(1))
    }
}

pub struct MockAddressMapping;
impl AddressMapping<AccountId32> for MockAddressMapping {
    fn into_account_id(address: H160) -> AccountId32 {
        if address == ALICE_H160 {
            return ALICE;
        }
        if address == BOB_H160 {
            return BOB;
        }
        if address == CHARLIE_H160 {
            return CHARLIE;
        }

        return pallet_evm::HashedAddressMapping::<BlakeTwo256>::into_account_id(address);
    }
}

pub struct MockAccountMapping;
impl AccountMapping<AccountId32> for MockAccountMapping {
    fn into_h160(account_id: AccountId) -> H160 {
        if account_id == ALICE {
            return ALICE_H160;
        }
        if account_id == BOB {
            return BOB_H160;
        }
        if account_id == CHARLIE {
            return CHARLIE_H160;
        }

        let data = (b"evm:", account_id);
        return H160::from_slice(&data.using_encoded(blake2_256)[0..20]);
    }
}

parameter_types! {
    pub WeightPerGas: Weight = Weight::from_parts(1, 0);
    pub const BlockGasLimit: U256 = U256::MAX;
}

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = MockFeeCalculator;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<TestRuntime>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = MockAddressMapping;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = ();
    type PrecompilesValue = ();
    type ChainId = ConstU64<1024>;
    type OnChargeTransaction = ();
    type BlockGasLimit = BlockGasLimit;
    type OnCreate = ();
    type FindAuthor = MockFindAuthor;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<TestRuntime>;
    type GasLimitPovSizeRatio = ConstU64<4>;
}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
    pub TxWeightLimit: Weight = Weight::from_parts(u64::max_value(), 0);
}

impl pallet_ethereum_checked::Config for TestRuntime {
    type ReservedXcmpWeight = TxWeightLimit;
    type XvmTxWeightLimit = TxWeightLimit;
    type InvalidEvmTransactionError = pallet_ethereum::InvalidTransactionWrapper;
    type ValidatedTransaction = pallet_ethereum::ValidatedTransaction<Self>;
    type AccountMapping = MockAccountMapping;
    type XcmTransactOrigin = EnsureXcmEthereumTx<AccountId32>;
    type WeightInfo = ();
}

pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([2u8; 32]);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

construct_runtime!(
    pub struct TestRuntime
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Evm: pallet_evm,
        Ethereum: pallet_ethereum,
        EthereumChecked: pallet_ethereum_checked::{Pallet, Call, Origin<T>},
    }
);

pub const ALICE_H160: H160 = H160::repeat_byte(1);
pub const BOB_H160: H160 = H160::repeat_byte(2);
pub const CHARLIE_H160: H160 = H160::repeat_byte(3);

/* Testing contract

pragma solidity >=0.8.2 <0.9.0;

contract Storage {
    uint256 number;

    /**
     * @dev Store value in variable
     * @param num value to store
     */
    function store(uint256 num) public {
        number = num;
    }

    /**
     * @dev Return value
     * @return value of 'number'
     */
    function retrieve() public view returns (uint256){
        return number;
    }
}
*/
const STORAGE_CONTRACT: &str = "608060405234801561001057600080fd5b50610150806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100a1565b60405180910390f35b610073600480360381019061006e91906100ed565b61007e565b005b60008054905090565b8060008190555050565b6000819050919050565b61009b81610088565b82525050565b60006020820190506100b66000830184610092565b92915050565b600080fd5b6100ca81610088565b81146100d557600080fd5b50565b6000813590506100e7816100c1565b92915050565b600060208284031215610103576101026100bc565b5b6000610111848285016100d8565b9150509291505056fea2646970667358221220322c78243e61b783558509c9cc22cb8493dde6925aa5e89a08cdf6e22f279ef164736f6c63430008120033";

pub fn contract_address() -> H160 {
    H160::from_slice(&hex::decode("dfb975d018f03994a3b943808e3aa0964bd78463").unwrap())
}

pub struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![
                (ALICE, 1_000_000_000_000),
                (BOB, 1_000_000_000_000),
                (CHARLIE, 1_000_000_000_000),
            ],
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = TestExternalities::from(t);
        ext.execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Evm::create2(
                RuntimeOrigin::root(),
                ALICE_H160,
                hex::decode(STORAGE_CONTRACT).unwrap(),
                H256::zero(),
                U256::zero(),
                1_000_000,
                U256::one(),
                None,
                Some(U256::zero()),
                vec![],
            ));
            System::assert_last_event(RuntimeEvent::Evm(pallet_evm::Event::Created {
                address: contract_address(),
            }));
        });
        ext
    }
}
