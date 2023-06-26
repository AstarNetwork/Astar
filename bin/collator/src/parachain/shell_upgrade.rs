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

///! Special [`ParachainConsensus`] implementation that waits for the upgrade from
///! shell to a parachain runtime that implements Aura.
use astar_primitives::*;
use cumulus_client_consensus_common::{ParachainCandidate, ParachainConsensus};
use cumulus_primitives_core::relay_chain::{Hash as PHash, PersistedValidationData};
use futures::lock::Mutex;
use sc_consensus::{import_queue::Verifier as VerifierT, BlockImportParams, ForkChoiceStrategy};
use sp_api::ApiExt;
use sp_consensus::CacheKeyId;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_runtime::traits::Header as HeaderT;
use std::sync::Arc;

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
        mut block_import: BlockImportParams<Block, ()>,
    ) -> Result<
        (
            BlockImportParams<Block, ()>,
            Option<Vec<(CacheKeyId, Vec<u8>)>>,
        ),
        String,
    > {
        // Skip checks that include execution, if being told so or when importing only state.
        //
        // This is done for example when gap syncing and it is expected that the block after the gap
        // was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
        // Or when we are importing state only and can not verify the seal.
        if block_import.with_state() || block_import.state_action.skip_execution_checks() {
            // When we are importing only the state of a block, it will be the best block.
            block_import.fork_choice = Some(ForkChoiceStrategy::Custom(block_import.with_state()));
            return Ok((block_import, None));
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
