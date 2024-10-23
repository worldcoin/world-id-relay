pub mod signer;

use std::sync::Arc;

use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use eyre::Result;
use semaphore::Field;
use signer::{RelaySigner, Signer};
use tokio::sync::broadcast::Receiver;
use url::Url;

use crate::abi::IBridgedWorldID::IBridgedWorldIDInstance;

// Two Mainnet Blocks
pub const ROOT_PROPAGATION_BACKOFF: u64 = 24;

pub(crate) trait Relay {
    /// Subscribe to the stream of new Roots on L1.
    async fn subscribe_roots(&self, rx: Receiver<Field>) -> Result<()>;
}

pub enum Relayer {
    Evm(EVMRelay),
    Svm(SvmRelay),
}

impl Relay for Relayer {
    async fn subscribe_roots(&self, rx: Receiver<Field>) -> Result<()> {
        match self {
            Relayer::Evm(relay) => relay.subscribe_roots(rx).await,
            Relayer::Svm(_relay) => unimplemented!(),
        }
    }
}

pub struct EVMRelay {
    pub signer: Signer,
    pub world_id_address: Address,
    pub provider: Url,
}

impl EVMRelay {
    pub fn new(
        signer: Signer,
        world_id_address: Address,
        provider: Url,
    ) -> Self {
        Self {
            signer,
            world_id_address,
            provider,
        }
    }
}

impl Relay for EVMRelay {
    async fn subscribe_roots(&self, mut rx: Receiver<Field>) -> Result<()> {
        let l2_provider = ProviderBuilder::new().on_http(self.provider.clone());
        let world_id_instance = Arc::new(IBridgedWorldIDInstance::new(
            self.world_id_address,
            l2_provider,
        ));

        loop {
            let field = rx.recv().await?;
            let world_id = world_id_instance.clone();
            let latest = world_id.latestRoot().call().await?._0;

            if latest != field {
                tracing::trace!(new_root = ?field, latest_root =?latest, "Propagating root");
                self.signer.propagate_root().await?;
                // We sleep for 2 blocks, so we don't resend the same root prior to derivation of the message on L2.
                std::thread::sleep(std::time::Duration::from_secs(
                    ROOT_PROPAGATION_BACKOFF,
                ));
            }
        }
    }
}

pub struct SvmRelay;
