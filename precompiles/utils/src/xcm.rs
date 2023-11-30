// This file is part of Astar.

// Copyright 2019-2022 PureStake Inc.
// Copyright (C) 2022-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This file is part of Utils package, originally developed by Purestake Inc.
// Utils package used in Astar Network in terms of GPLv3.
//
// Utils is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Utils is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Utils.  If not, see <http://www.gnu.org/licenses/>.

//! Encoding of XCM types for solidity

use crate::Address;
use sp_core::U256;
use sp_runtime::traits::Zero;
use {
    crate::{bytes::*, revert, EvmData, EvmDataReader, EvmDataWriter, EvmResult},
    frame_support::{ensure, pallet_prelude::Weight, traits::ConstU32},
    sp_core::H256,
    sp_std::vec::Vec,
    xcm::latest::{Junction, Junctions, MultiLocation, NetworkId},
};
pub const JUNCTION_SIZE_LIMIT: u32 = 2u32.pow(16);

// Function to convert network id to bytes
// Each NetworkId variant is represented as bytes
// The first byte represents the enum variant to be used.
// 		- Indexes 0,2,3 represent XCM V2 variants
// 		- Index 1 changes name in V3 (`ByGenesis`), but is compatible with V2 `Named`
// 		- Indexes 4~10 represent new XCM V3 variants
// The rest of the bytes (if any), represent the additional data that such enum variant requires
// In such a case, since NetworkIds will be appended at the end, we will read the buffer until the
// end to recover the name

pub(crate) fn network_id_to_bytes(network_id: Option<NetworkId>) -> Vec<u8> {
    let mut encoded: Vec<u8> = Vec::new();
    match network_id.clone() {
        None => {
            encoded.push(0u8);
            encoded
        }
        Some(NetworkId::ByGenesis(id)) => {
            encoded.push(1u8);
            encoded.append(&mut id.into());
            encoded
        }
        Some(NetworkId::Polkadot) => {
            encoded.push(2u8);
            encoded.push(2u8);
            encoded
        }
        Some(NetworkId::Kusama) => {
            encoded.push(3u8);
            encoded.push(3u8);
            encoded
        }
        Some(NetworkId::ByFork {
            block_number,
            block_hash,
        }) => {
            encoded.push(4u8);
            encoded.push(1u8);
            encoded.append(&mut block_number.to_be_bytes().into());
            encoded.append(&mut block_hash.into());
            encoded
        }
        Some(NetworkId::Westend) => {
            encoded.push(5u8);
            encoded.push(4u8);
            encoded
        }
        Some(NetworkId::Rococo) => {
            encoded.push(6u8);
            encoded.push(5u8);
            encoded
        }
        Some(NetworkId::Wococo) => {
            encoded.push(7u8);
            encoded.push(6u8);
            encoded
        }
        Some(NetworkId::Ethereum { chain_id }) => {
            encoded.push(8u8);
            encoded.push(7u8);
            encoded.append(&mut chain_id.to_be_bytes().into());
            encoded
        }
        Some(NetworkId::BitcoinCore) => {
            encoded.push(9u8);
            encoded.push(8u8);
            encoded
        }
        Some(NetworkId::BitcoinCash) => {
            encoded.push(10u8);
            encoded.push(9u8);
            encoded
        }
    }
}

// Function to convert bytes to networkId
pub(crate) fn network_id_from_bytes(encoded_bytes: Vec<u8>) -> EvmResult<Option<NetworkId>> {
    ensure!(encoded_bytes.len() > 0, revert("Junctions cannot be empty"));
    let mut encoded_network_id = EvmDataReader::new(&encoded_bytes);

    let network_selector = encoded_network_id
        .read_raw_bytes(1)
        .map_err(|_| revert("network selector (1 byte)"))?;

    match network_selector[0] {
        0 => Ok(None),
        1 => Ok(Some(NetworkId::ByGenesis(
            encoded_network_id
                .read_till_end()
                .map_err(|_| revert("can't read till end"))?
                .to_vec()
                .try_into()
                .map_err(|_| revert("network by genesis"))?,
        ))),
        2 => Ok(Some(NetworkId::Polkadot)),
        3 => Ok(Some(NetworkId::Kusama)),
        4 => {
            let mut block_number: [u8; 8] = Default::default();
            block_number.copy_from_slice(&encoded_network_id.read_raw_bytes(8)?);

            let mut block_hash: [u8; 32] = Default::default();
            block_hash.copy_from_slice(&encoded_network_id.read_raw_bytes(32)?);
            Ok(Some(NetworkId::ByFork {
                block_number: u64::from_be_bytes(block_number),
                block_hash,
            }))
        }
        5 => Ok(Some(NetworkId::Westend)),
        6 => Ok(Some(NetworkId::Rococo)),
        7 => Ok(Some(NetworkId::Wococo)),
        8 => {
            let mut chain_id: [u8; 8] = Default::default();
            chain_id.copy_from_slice(&encoded_network_id.read_raw_bytes(8)?);
            Ok(Some(NetworkId::Ethereum {
                chain_id: u64::from_be_bytes(chain_id),
            }))
        }
        9 => Ok(Some(NetworkId::BitcoinCore)),
        10 => Ok(Some(NetworkId::BitcoinCash)),
        _ => Err(revert("Non-valid Network Id").into()),
    }
}

impl EvmData for Junction {
    fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
        let junction = reader.read::<BoundedBytes<ConstU32<JUNCTION_SIZE_LIMIT>>>()?;
        let junction_bytes: Vec<_> = junction.into();

        ensure!(
            junction_bytes.len() > 0,
            revert("Junctions cannot be empty")
        );

        // For simplicity we use an EvmReader here
        let mut encoded_junction = EvmDataReader::new(&junction_bytes);

        // We take the first byte
        let enum_selector = encoded_junction
            .read_raw_bytes(1)
            .map_err(|_| revert("junction variant"))?;

        // The firs byte selects the enum variant
        match enum_selector[0] {
            0 => {
                // In the case of Junction::Parachain, we need 4 additional bytes
                let mut data: [u8; 4] = Default::default();
                data.copy_from_slice(&encoded_junction.read_raw_bytes(4)?);
                let para_id = u32::from_be_bytes(data);
                Ok(Junction::Parachain(para_id))
            }
            1 => {
                // In the case of Junction::AccountId32, we need 32 additional bytes plus NetworkId
                let mut account: [u8; 32] = Default::default();
                account.copy_from_slice(&encoded_junction.read_raw_bytes(32)?);

                let network = encoded_junction.read_till_end()?.to_vec();
                Ok(Junction::AccountId32 {
                    network: network_id_from_bytes(network)?,
                    id: account,
                })
            }
            2 => {
                // In the case of Junction::AccountIndex64, we need 8 additional bytes plus NetworkId
                let mut index: [u8; 8] = Default::default();
                index.copy_from_slice(&encoded_junction.read_raw_bytes(8)?);
                // Now we read the network
                let network = encoded_junction.read_till_end()?.to_vec();
                Ok(Junction::AccountIndex64 {
                    network: network_id_from_bytes(network)?,
                    index: u64::from_be_bytes(index),
                })
            }
            3 => {
                // In the case of Junction::AccountKey20, we need 20 additional bytes plus NetworkId
                let mut account: [u8; 20] = Default::default();
                account.copy_from_slice(&encoded_junction.read_raw_bytes(20)?);

                let network = encoded_junction.read_till_end()?.to_vec();
                Ok(Junction::AccountKey20 {
                    network: network_id_from_bytes(network)?,
                    key: account,
                })
            }
            4 => Ok(Junction::PalletInstance(
                encoded_junction.read_raw_bytes(1)?[0],
            )),
            5 => {
                // In the case of Junction::GeneralIndex, we need 16 additional bytes
                let mut general_index: [u8; 16] = Default::default();
                general_index.copy_from_slice(&encoded_junction.read_raw_bytes(16)?);
                Ok(Junction::GeneralIndex(u128::from_be_bytes(general_index)))
            }
            6 => {
                let length = encoded_junction
                    .read_raw_bytes(1)
                    .map_err(|_| revert("General Key length"))?[0];

                let data = encoded_junction
                    .read::<H256>()
                    .map_err(|_| revert("can't read"))?
                    .into();

                Ok(Junction::GeneralKey { length, data })
            }
            7 => Ok(Junction::OnlyChild),
            8 => Err(revert("Junction::Plurality not supported yet").into()),
            9 => {
                let network = encoded_junction.read_till_end()?.to_vec();
                if let Some(network_id) = network_id_from_bytes(network)? {
                    Ok(Junction::GlobalConsensus(network_id))
                } else {
                    Err(revert("Unknown NetworkId").into())
                }
            }
            _ => Err(revert("Unknown Junction variant").into()),
        }
    }

    fn write(writer: &mut EvmDataWriter, value: Self) {
        let mut encoded: Vec<u8> = Vec::new();
        let encoded_bytes: UnboundedBytes = match value {
            Junction::Parachain(para_id) => {
                encoded.push(0u8);
                encoded.append(&mut para_id.to_be_bytes().to_vec());
                encoded.as_slice().into()
            }
            Junction::AccountId32 { network, id } => {
                encoded.push(1u8);
                encoded.append(&mut id.to_vec());
                encoded.append(&mut network_id_to_bytes(network));
                encoded.as_slice().into()
            }
            Junction::AccountIndex64 { network, index } => {
                encoded.push(2u8);
                encoded.append(&mut index.to_be_bytes().to_vec());
                encoded.append(&mut network_id_to_bytes(network));
                encoded.as_slice().into()
            }
            Junction::AccountKey20 { network, key } => {
                encoded.push(3u8);
                encoded.append(&mut key.to_vec());
                encoded.append(&mut network_id_to_bytes(network));
                encoded.as_slice().into()
            }
            Junction::PalletInstance(intance) => {
                encoded.push(4u8);
                encoded.append(&mut intance.to_be_bytes().to_vec());
                encoded.as_slice().into()
            }
            Junction::GeneralIndex(id) => {
                encoded.push(5u8);
                encoded.append(&mut id.to_be_bytes().to_vec());
                encoded.as_slice().into()
            }
            Junction::GeneralKey { length, data } => {
                encoded.push(6u8);
                encoded.push(length);
                encoded.append(&mut data.into());
                encoded.as_slice().into()
            }
            Junction::OnlyChild => {
                encoded.push(7u8);
                encoded.as_slice().into()
            }
            Junction::GlobalConsensus(network_id) => {
                encoded.push(9u8);
                encoded.append(&mut network_id_to_bytes(Some(network_id)));
                encoded.as_slice().into()
            }
            // TODO: The only missing item here is Junciton::Plurality. This is a complex encoded
            // type that we need to evaluate how to support
            _ => unreachable!("Junction::Plurality not supported yet"),
        };
        EvmData::write(writer, encoded_bytes);
    }

    fn has_static_size() -> bool {
        false
    }
}

impl EvmData for Junctions {
    fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
        let junctions_bytes: Vec<Junction> = reader.read()?;
        let mut junctions = Junctions::Here;
        for item in junctions_bytes {
            junctions
                .push(item)
                .map_err(|_| revert("overflow when reading junctions"))?;
        }

        Ok(junctions)
    }

    fn write(writer: &mut EvmDataWriter, value: Self) {
        let encoded: Vec<Junction> = value.iter().map(|junction| junction.clone()).collect();
        EvmData::write(writer, encoded);
    }

    fn has_static_size() -> bool {
        false
    }
}

// Cannot used derive macro since it is a foreign struct.
impl EvmData for MultiLocation {
    fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
        let (parents, interior) = reader.read()?;
        Ok(MultiLocation { parents, interior })
    }

    fn write(writer: &mut EvmDataWriter, value: Self) {
        EvmData::write(writer, (value.parents, value.interior));
    }

    fn has_static_size() -> bool {
        <(u8, Junctions)>::has_static_size()
    }
}

#[derive(Debug, Clone)]
pub struct WeightV2 {
    ref_time: u64,
    proof_size: u64,
}
impl WeightV2 {
    pub fn from(ref_time: u64, proof_size: u64) -> Self {
        WeightV2 {
            ref_time,
            proof_size,
        }
    }
    pub fn get_weight(&self) -> Weight {
        Weight::from_parts(self.ref_time, self.proof_size)
    }
    pub fn is_zero(&self) -> bool {
        self.ref_time.is_zero()
    }
}
impl EvmData for WeightV2 {
    fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
        let (ref_time, proof_size) = reader.read()?;
        Ok(WeightV2 {
            ref_time,
            proof_size,
        })
    }

    fn write(writer: &mut EvmDataWriter, value: Self) {
        EvmData::write(writer, (value.ref_time, value.proof_size));
    }

    fn has_static_size() -> bool {
        <(U256, U256)>::has_static_size()
    }
}
#[derive(Debug)]
pub struct EvmMultiAsset {
    location: MultiLocation,
    amount: U256,
}

impl EvmMultiAsset {
    pub fn get_location(&self) -> MultiLocation {
        self.location
    }
    pub fn get_amount(&self) -> U256 {
        self.amount
    }
}
impl From<(MultiLocation, U256)> for EvmMultiAsset {
    fn from(tuple: (MultiLocation, U256)) -> Self {
        EvmMultiAsset {
            location: tuple.0,
            amount: tuple.1,
        }
    }
}
impl EvmData for EvmMultiAsset {
    fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
        let (location, amount) = reader.read()?;
        Ok(EvmMultiAsset { location, amount })
    }

    fn write(writer: &mut EvmDataWriter, value: Self) {
        EvmData::write(writer, (value.location, value.amount));
    }

    fn has_static_size() -> bool {
        <(MultiLocation, U256)>::has_static_size()
    }
}

pub struct Currency {
    address: Address,
    amount: U256,
}

impl Currency {
    pub fn get_address(&self) -> Address {
        self.address
    }
    pub fn get_amount(&self) -> U256 {
        self.amount
    }
}
impl From<(Address, U256)> for Currency {
    fn from(tuple: (Address, U256)) -> Self {
        Currency {
            address: tuple.0,
            amount: tuple.1,
        }
    }
}

impl EvmData for Currency {
    fn read(reader: &mut EvmDataReader) -> EvmResult<Self> {
        let (address, amount) = reader.read()?;
        Ok(Currency { address, amount })
    }

    fn write(writer: &mut EvmDataWriter, value: Self) {
        EvmData::write(writer, (value.address, value.amount));
    }

    fn has_static_size() -> bool {
        <(Address, U256)>::has_static_size()
    }
}
