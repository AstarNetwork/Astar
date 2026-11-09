// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

//! Utility for the upgrade from shell to a parachain runtime that implements Aura.
use astar_primitives::*;
use cumulus_primitives_core::relay_chain::{self, PersistedValidationData};
use cumulus_primitives_core::{ParaId, RelayParentOffsetApi};
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use fc_rpc::pending::ConsensusDataProvider;
use parity_scale_codec::{Decode, Encode};
use sc_client_api::{AuxStore, Backend, StorageProvider, UsageProvider};
use sc_consensus::{import_queue::Verifier as VerifierT, BlockImportParams};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::{
    digests::CompatibleDigestItem,
    sr25519::{AuthorityId as AuraId, AuthoritySignature},
    AuraApi,
};
use sp_core::{hashing::twox_128, storage::StorageKey};
use sp_inherents::{CreateInherentDataProviders, Error, InherentData};
use sp_runtime::{
    traits::{Block as BlockT, Header as HeaderT},
    Digest, DigestItem,
};
use sp_timestamp::TimestampInherentData;
use std::{marker::PhantomData, sync::Arc};

pub struct Verifier<Client> {
    pub client: Arc<Client>,
    pub aura_verifier: Box<dyn VerifierT<Block>>,
    pub relay_chain_verifier: Box<dyn VerifierT<Block>>,
}

#[async_trait::async_trait]
impl<Client> VerifierT<Block> for Verifier<Client>
where
    Client: ProvideRuntimeApi<Block> + Send + Sync,
    Client::Api: AuraApi<Block, AuraId>,
{
    async fn verify(
        &self,
        block_import: BlockImportParams<Block>,
    ) -> Result<BlockImportParams<Block>, String> {
        if self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(*block_import.header.parent_hash())
            .unwrap_or(false)
        {
            self.aura_verifier.verify(block_import).await
        } else {
            self.relay_chain_verifier.verify(block_import).await
        }
    }
}

/// AuraConsensusDataProvider custom implementation which awaits for AuraApi to become available,
/// until then it will return error. Shiden genesis did not start with AuraApi, therefore this
/// implementation makes sure to return digest after AuraApi becomes available.
/// This is currently required by EVM RPC.
pub struct AuraConsensusDataProviderFallback<B, C> {
    client: Arc<C>,
    phantom_data: PhantomData<B>,
}

impl<B, C> AuraConsensusDataProviderFallback<B, C>
where
    B: BlockT,
    C: AuxStore + ProvideRuntimeApi<B> + UsageProvider<B> + Send + Sync,
    C::Api: AuraApi<B, AuraId>,
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            phantom_data: Default::default(),
        }
    }
}

impl<B, C> ConsensusDataProvider<B> for AuraConsensusDataProviderFallback<B, C>
where
    B: BlockT,
    C: AuxStore + ProvideRuntimeApi<B> + UsageProvider<B> + Send + Sync,
    C::Api: AuraApi<B, AuraId>,
{
    fn create_digest(&self, parent: &B::Header, data: &InherentData) -> Result<Digest, Error> {
        if self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(parent.hash())
            .unwrap_or_default()
        {
            let slot_duration = sc_consensus_aura::slot_duration(&*self.client)
                .expect("slot_duration should be present at this point; qed.");
            let timestamp = data
                .timestamp_inherent_data()?
                .expect("Timestamp is always present; qed");

            let digest_item =
                <DigestItem as CompatibleDigestItem<AuthoritySignature>>::aura_pre_digest(
                    sp_consensus_aura::Slot::from_timestamp(timestamp, slot_duration),
                );

            return Ok(Digest {
                logs: vec![digest_item],
            });
        }
        Err(Error::Application("AuraApi is not present".into()))
    }
}

/// Relay chain slot duration, used to mock a relay slot matching the pending block's para slot.
const RELAY_CHAIN_SLOT_DURATION_MILLIS: u64 = 6000;

/// Shiden genesis did not start with AuraApi, therefore this implementation makes sure to return
/// inherent data after AuraApi becomes available.
/// This is currently required by EVM RPC.
pub struct PendingCrateInherentDataProvider<B, C, BE> {
    client: Arc<C>,
    para_id: ParaId,
    phantom_data: PhantomData<fn() -> (B, BE)>,
}

impl<B, C, BE> PendingCrateInherentDataProvider<B, C, BE> {
    pub fn new(client: Arc<C>, para_id: ParaId) -> Self {
        Self {
            client,
            para_id,
            phantom_data: Default::default(),
        }
    }
}

impl<B, C, BE> PendingCrateInherentDataProvider<B, C, BE>
where
    B: BlockT,
    BE: Backend<B>,
    C: StorageProvider<B, BE>,
{
    /// Reads `ParachainSystem::LastDmqMqcHead` at `parent`, so the mocked DMQ head matches it.
    fn last_dmq_mqc_head(
        &self,
        parent: B::Hash,
    ) -> Result<relay_chain::Hash, Box<dyn std::error::Error + Send + Sync>> {
        let key = [twox_128(b"ParachainSystem"), twox_128(b"LastDmqMqcHead")].concat();
        Ok(self
            .client
            .storage(parent, &StorageKey(key))?
            .map(|data| relay_chain::Hash::decode(&mut &data.0[..]))
            .transpose()?
            .unwrap_or_default())
    }
}

#[async_trait::async_trait]
impl<B, C, BE> CreateInherentDataProviders<B, ()> for PendingCrateInherentDataProvider<B, C, BE>
where
    B: BlockT,
    BE: Backend<B>,
    C: AuxStore
        + ProvideRuntimeApi<B>
        + UsageProvider<B>
        + HeaderBackend<B>
        + StorageProvider<B, BE>
        + Send
        + Sync,
    C::Api: AuraApi<B, AuraId> + RelayParentOffsetApi<B>,
{
    type InherentDataProviders = (
        sp_consensus_aura::inherents::InherentDataProvider,
        sp_timestamp::InherentDataProvider,
        cumulus_primitives_parachain_inherent::ParachainInherentData,
    );

    async fn create_inherent_data_providers(
        &self,
        parent: B::Hash,
        _extra_args: (),
    ) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
        if !self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(parent)
            .unwrap_or_default()
        {
            return Err("AuraApi is not present".into());
        }

        let slot_duration = sc_consensus_aura::slot_duration(&*self.client)
            .expect("slot_duration should be present at this point; qed.");
        let current = sp_timestamp::InherentDataProvider::from_system_time();
        let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
        let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
        let slot =
            sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
        // Mock relay state that passes the parachain-system and aura-ext checks on top of `parent`.
        let sproof_builder = RelayStateSproofBuilder {
            para_id: self.para_id,
            current_slot: (next_slot / RELAY_CHAIN_SLOT_DURATION_MILLIS).into(),
            included_para_head: self
                .client
                .header(parent)?
                .map(|header| header.encode().into()),
            dmq_mqc_head: Some(self.last_dmq_mqc_head(parent)?),
            ..Default::default()
        };
        let (relay_parent_storage_root, relay_chain_state, relay_parent_descendants) = if self
            .client
            .runtime_api()
            .has_api::<dyn RelayParentOffsetApi<B>>(parent)
            .unwrap_or(false)
        {
            let offset = self
                .client
                .runtime_api()
                .relay_parent_offset(parent)
                .expect("API present, call should succeed; qed");
            sproof_builder.into_state_root_proof_and_descendants(offset.into())
        } else {
            let (root, proof) = sproof_builder.into_state_root_and_proof();
            (root, proof, Default::default())
        };
        let vfp = PersistedValidationData {
            // This is a hack to make `cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases`
            // happy. Relay parent number can't be bigger than u32::MAX.
            relay_parent_number: u32::MAX,
            relay_parent_storage_root,
            ..Default::default()
        };
        let parachain_inherent_data =
            cumulus_primitives_parachain_inherent::ParachainInherentData {
                validation_data: vfp,
                relay_chain_state,
                downward_messages: Default::default(),
                horizontal_messages: Default::default(),
                relay_parent_descendants,
                collator_peer_id: Default::default(),
            };
        Ok((slot, timestamp, parachain_inherent_data))
    }
}
