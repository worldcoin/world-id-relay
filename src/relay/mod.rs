pub mod signer;

use std::{sync::Arc, time::Duration};

use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
};
use eyre::Result;
use signer::{RelaySigner, Signer};
use tokio::sync::broadcast::Receiver;
use url::Url;

use crate::{
    abi::IBridgedWorldID::IBridgedWorldIDInstance,
    utils::{retry, RetryConfig},
};

static RETRY_CONFIG: RetryConfig = RetryConfig {
    min_delay: Duration::from_millis(10),
    max_delay: Duration::from_secs(300),
    max_times: 50,
};

// Two Mainnet Blocks
pub const ROOT_PROPAGATION_BACKOFF: u64 = 24;

pub(crate) trait Relay {
    /// Subscribe to the stream of new Roots on L1.
    async fn subscribe_roots(&self, rx: Receiver<U256>) -> Result<()>;
}

macro_rules! relay {
    ($($relay_type:ident),+ $(,)?) => {
        pub enum Relayer {
            $($relay_type($relay_type),)+
        }
        impl Relay for Relayer {
            async fn subscribe_roots(&self, rx: Receiver<U256>) -> Result<()> {
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
    async fn subscribe_roots(&self, mut rx: Receiver<U256>) -> Result<()> {
        let l2_provider = ProviderBuilder::new().on_http(self.provider.clone());
        let world_id_instance = Arc::new(IBridgedWorldIDInstance::new(
            self.world_id_address,
            l2_provider,
        ));

        loop {
            let field = rx.recv().await?;
            let world_id = world_id_instance.clone();

            retry(
                || async {
                    let latest = world_id.latestRoot().call().await?._0;

                    if latest != field {
                        self.signer.propagate_root().await?;
                    }

                    tracing::info!(root = %field, previous_root=%latest, provider = %self.provider, "Root propagated successfully");

                    Ok::<(), eyre::Report>(())
                },
                &RETRY_CONFIG,
                "Failed to propagate root, retrying",
                "Failed to propagate root",
            )
            .await?;
        }
    }
}

pub struct SvmRelay;

impl Relay for SvmRelay {
    async fn subscribe_roots(&self, _rx: Receiver<U256>) -> Result<()> {
        unimplemented!()
    }
}

relay!(EVMRelay, SvmRelay);
