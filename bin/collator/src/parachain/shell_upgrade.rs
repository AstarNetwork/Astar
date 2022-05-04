///! Special [`ParachainConsensus`] implementation that waits for the upgrade from
///! shell to a parachain runtime that implements Aura.
use cumulus_client_consensus_common::{ParachainCandidate, ParachainConsensus};
use cumulus_primitives_core::relay_chain::v2::{Hash as PHash, PersistedValidationData};
use futures::lock::Mutex;
use sc_consensus::{import_queue::Verifier as VerifierT, BlockImportParams};
use sp_api::ApiExt;
use sp_consensus::CacheKeyId;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_runtime::{generic::BlockId, traits::Header as HeaderT};
use std::sync::Arc;

use crate::primitives::*;

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
        let block_id = BlockId::hash(parent.hash());
        if self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(&block_id)
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
        block_import: BlockImportParams<Block, ()>,
    ) -> Result<
        (
            BlockImportParams<Block, ()>,
            Option<Vec<(CacheKeyId, Vec<u8>)>>,
        ),
        String,
    > {
        let block_id = BlockId::hash(*block_import.header.parent_hash());

        if self
            .client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(&block_id)
            .unwrap_or(false)
        {
            self.aura_verifier.get_mut().verify(block_import).await
        } else {
            self.relay_chain_verifier.verify(block_import).await
        }
    }
}
