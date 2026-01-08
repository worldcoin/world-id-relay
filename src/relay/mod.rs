pub mod signer;

use std::sync::Arc;
use std::time::Duration;

use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use eyre::Result;
use semaphore::Field;
use signer::{RelaySigner, Signer};
use tokio::sync::broadcast::Receiver;
use url::Url;

use crate::abi::IBridgedWorldID::IBridgedWorldIDInstance;

const MAX_RETRIES: u64 = 50;

/// How long we should wait before retrying a failed `relayRoot` request
pub const RELAY_ROOT_RETRY_BACKOFF: Duration = Duration::from_millis(1000);

// Two Mainnet Blocks
pub const ROOT_PROPAGATION_BACKOFF: u64 = 24;

pub(crate) trait Relay {
    /// Subscribe to the stream of new Roots on L1.
    async fn subscribe_roots(&self, rx: Receiver<Field>) -> Result<()>;
}

macro_rules! relay {
    ($($relay_type:ident),+ $(,)?) => {
        pub enum Relayer {
            $($relay_type($relay_type),)+
        }
        impl Relay for Relayer {
            async fn subscribe_roots(&self, rx: Receiver<Field>) -> Result<()> {
                match self {
                    $(Relayer::$relay_type(relay) => Ok(relay.subscribe_roots(rx).await?),)+
                }
            }
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

        let propagate_root = || async { self.signer.propagate_root().await };

        loop {
            let field = rx.recv().await?;
            // we sit here until a new `RootUpdated` event
            let world_id = world_id_instance.clone();
            let latest = world_id.latestRoot().call().await?._0;

            if latest != field {
                match propagate_root().await {
                    Ok(_) => {
                        tracing::info!(root = %field, previous_root=%latest, provider = %self.provider, "Root propagated successfully");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, root = %field, previous_root=%latest, total_retries = 0, provider = %self.provider, "Failed to propagate root");

                        let mut total_retries = 0;
                        while total_retries < MAX_RETRIES {
                            match propagate_root().await {
                                Ok(_) => {
                                    break;
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, root = %field, previous_root=%latest, total_retries = %total_retries, provider = %self.provider, "Failed to propagate root");
                                    total_retries += 1;
                                    tokio::time::sleep(
                                        RELAY_ROOT_RETRY_BACKOFF,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }
                // We sleep for 2 blocks, so we don't resend the same root prior to derivation of the message on L2.
                std::thread::sleep(std::time::Duration::from_secs(
                    ROOT_PROPAGATION_BACKOFF,
                ));
            }
        }
    }
}

pub struct SvmRelay;

impl Relay for SvmRelay {
    async fn subscribe_roots(&self, _rx: Receiver<Field>) -> Result<()> {
        unimplemented!()
    }
}

relay!(EVMRelay, SvmRelay);
