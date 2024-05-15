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

///! Special [`ParachainConsensus`] implementation that waits for the upgrade from
///! shell to a parachain runtime that implements Aura.
use astar_primitives::*;
use cumulus_client_consensus_common::{ParachainCandidate, ParachainConsensus};
use cumulus_primitives_core::relay_chain::{Hash as PHash, PersistedValidationData};
use cumulus_primitives_core::BlockT;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use fc_rpc::pending::ConsensusDataProvider;
use futures::lock::Mutex;
use sc_client_api::{AuxStore, UsageProvider};
use sc_consensus::{import_queue::Verifier as VerifierT, BlockImportParams, ForkChoiceStrategy};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_consensus_aura::digests::CompatibleDigestItem;
use sp_consensus_aura::sr25519::AuthoritySignature;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_inherents::{CreateInherentDataProviders, Error, InherentData};
use sp_runtime::traits::Header as HeaderT;
use sp_runtime::{Digest, DigestItem};
use sp_timestamp::TimestampInherentData;
use std::{marker::PhantomData, sync::Arc};

pub enum BuildOnAccess<R> {
    Uninitialized(Option<Box<dyn FnOnce() -> R + Send + Sync>>),
    Initialized(R),
}

impl<R> BuildOnAccess<R> {
    fn get_mut(&mut self) -> &mut R {
        loop {
            match self {
                Self::Uninitialized(f) => {
                    *self = Self::Initialized((f.take().unwrap())());
                }
                Self::Initialized(ref mut r) => return r,
            }
        }
    }
}

pub struct WaitForAuraConsensus<Client> {
    pub client: Arc<Client>,
    pub aura_consensus: Arc<Mutex<BuildOnAccess<Box<dyn ParachainConsensus<Block>>>>>,
    pub relay_chain_consensus: Arc<Mutex<Box<dyn ParachainConsensus<Block>>>>,
}

impl<Client> Clone for WaitForAuraConsensus<Client> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            aura_consensus: self.aura_consensus.clone(),
            relay_chain_consensus: self.relay_chain_consensus.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<Client> ParachainConsensus<Block> for WaitForAuraConsensus<Client>
where
    Client: sp_api::ProvideRuntimeApi<Block> + Send + Sync,
    Client::Api: AuraApi<Block, AuraId>,
{
    async fn produce_candidate(
        &mut self,
        parent: &Header,
        relay_parent: PHash,
        validation_data: &PersistedValidationData,
    ) -> Option<ParachainCandidate<Block>> {
        let block_hash = parent.hash();
        if self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(block_hash)
            .unwrap_or(false)
        {
            self.aura_consensus
                .lock()
                .await
                .get_mut()
                .produce_candidate(parent, relay_parent, validation_data)
                .await
        } else {
            self.relay_chain_consensus
                .lock()
                .await
                .produce_candidate(parent, relay_parent, validation_data)
                .await
        }
    }
}

pub struct Verifier<Client> {
    pub client: Arc<Client>,
    pub aura_verifier: BuildOnAccess<Box<dyn VerifierT<Block>>>,
    pub relay_chain_verifier: Box<dyn VerifierT<Block>>,
}

#[async_trait::async_trait]
impl<Client> VerifierT<Block> for Verifier<Client>
where
    Client: sp_api::ProvideRuntimeApi<Block> + Send + Sync,
    Client::Api: AuraApi<Block, AuraId>,
{
    async fn verify(
        &mut self,
        mut block_import: BlockImportParams<Block>,
    ) -> Result<BlockImportParams<Block>, String> {
        // Skip checks that include execution, if being told so or when importing only state.
        //
        // This is done for example when gap syncing and it is expected that the block after the gap
        // was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
        // Or when we are importing state only and can not verify the seal.
        if block_import.with_state() || block_import.state_action.skip_execution_checks() {
            // When we are importing only the state of a block, it will be the best block.
            block_import.fork_choice = Some(ForkChoiceStrategy::Custom(block_import.with_state()));
            return Ok(block_import);
        }

        let block_hash = *block_import.header.parent_hash();

        if self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(block_hash)
            .unwrap_or(false)
        {
            self.aura_verifier.get_mut().verify(block_import).await
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

/// Shiden genesis did not start with AuraApi, therefore this implementation makes sure to return
/// inherent data after AuraApi becomes available.
/// This is currently required by EVM RPC.
pub struct PendingCrateInherentDataProvider<B, C> {
    client: Arc<C>,
    phantom_data: PhantomData<B>,
}

impl<B, C> PendingCrateInherentDataProvider<B, C>
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

#[async_trait::async_trait]
impl<B, C> CreateInherentDataProviders<B, ()> for PendingCrateInherentDataProvider<B, C>
where
    B: BlockT,
    C: AuxStore + ProvideRuntimeApi<B> + UsageProvider<B> + Send + Sync,
    C::Api: AuraApi<B, AuraId>,
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
        // Create a dummy parachain inherent data provider which is required to pass
        // the checks by the para chain system. We use dummy values because in the 'pending context'
        // neither do we have access to the real values nor do we need them.
        let (relay_parent_storage_root, relay_chain_state) =
            RelayStateSproofBuilder::default().into_state_root_and_proof();
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
            };
        Ok((slot, timestamp, parachain_inherent_data))
    }
}
