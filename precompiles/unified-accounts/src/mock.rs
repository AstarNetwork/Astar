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

use super::*;

use fp_evm::{IsPrecompileResult, Precompile};
use frame_support::{construct_runtime, parameter_types, traits::ConstU64, weights::Weight};
pub use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{keccak_256, H160, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, ConstU32, IdentityLookup},
    AccountId32,
};

use ethers::{
    contract::{Eip712, EthAbiType},
    core::types::transaction::eip712::Eip712,
};

use astar_primitives::evm::HashedDefaultMappings;
pub type AccountId = AccountId32;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;

pub const PRECOMPILE_ADDRESS: H160 = H160::repeat_byte(0x7B);

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);

pub fn alice_secret() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}
/// EIP712 Payload struct
#[derive(Eip712, EthAbiType, Clone)]
#[eip712(
        name = "Astar EVM Claim",
        version = "1",
        chain_id = 1024,
        // mock genisis hash
        raw_salt = "0x4545454545454545454545454545454545454545454545454545454545454545"
    )]
struct Claim {
    substrate_address: ethers::core::types::Bytes,
}

/// Build the signature payload for given native account and eth private key
pub fn get_evm_signature(who: &AccountId32, secret: &libsecp256k1::SecretKey) -> [u8; 65] {
    // sign the payload
    UnifiedAccounts::eth_sign_prehash(
        &Claim {
            substrate_address: who.encode().into(),
        }
        .encode_eip712()
        .unwrap(),
        secret,
    )
}

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    TypeInfo,
)]
pub enum TestAccount {
    Viktor,
    Precompile,
}
impl Default for TestAccount {
    fn default() -> Self {
        Self::Viktor
    }
}
impl From<TestAccount> for H160 {
    fn from(value: TestAccount) -> H160 {
        match value {
            TestAccount::Viktor => H160::repeat_byte(0xAA),
            TestAccount::Precompile => PRECOMPILE_ADDRESS,
        }
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
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
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<AccountId32>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
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
#[derive(Debug, Clone, Copy)]
pub struct TestPrecompileSet<R>(PhantomData<R>);

impl<R> PrecompileSet for TestPrecompileSet<R>
where
    R: pallet_evm::Config,
    UnifiedAccountsPrecompile<R, pallet_unified_accounts::Pallet<R>>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == PRECOMPILE_ADDRESS => Some(UnifiedAccountsPrecompile::<
                R,
                pallet_unified_accounts::Pallet<R>,
            >::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: address == PRECOMPILE_ADDRESS,
            extra_cost: 0,
        }
    }
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for TestRuntime {
    type MaxReserves = ();
    type ReserveIdentifier = ();
    type MaxLocks = ();
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const PrecompilesValue: TestPrecompileSet<TestRuntime> =
        TestPrecompileSet(PhantomData);
    pub WeightPerGas: Weight = Weight::from_parts(1, 0);
}

pub type PrecompileCall = UnifiedAccountsPrecompileCall<TestRuntime, UnifiedAccounts>;

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = UnifiedAccounts;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = TestPrecompileSet<Self>;
    type PrecompilesValue = PrecompilesValue;
    type Timestamp = Timestamp;
    type ChainId = ChainId;
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type WeightInfo = ();
    type GasLimitPovSizeRatio = ConstU64<4>;
}

parameter_types! {
    // 2 storage items with value size 20 and 32
    pub const AccountMappingStorageFee: u128 = 0;
    pub ChainId: u64 = 1024;
}

impl pallet_unified_accounts::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type DefaultMappings = HashedDefaultMappings<BlakeTwo256>;
    type ChainId = ChainId;
    type AccountMappingStorageFee = AccountMappingStorageFee;
    type WeightInfo = pallet_unified_accounts::weights::SubstrateWeight<Self>;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Evm: pallet_evm,
        UnifiedAccounts: pallet_unified_accounts,
        Balances : pallet_balances,
        Timestamp: pallet_timestamp,
    }
);

#[derive(Default)]
pub(crate) struct ExtBuilder;

impl ExtBuilder {
    pub(crate) fn build(self) -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .expect("Frame system builds valid default genesis config");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
