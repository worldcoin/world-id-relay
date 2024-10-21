pub mod signer;

use std::marker::PhantomData;
use std::sync::Arc;

use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use eyre::Result;
use semaphore::Field;
use signer::RelaySigner;
use tokio::sync::mpsc::Receiver;
use url::Url;

use crate::abi::IBridgedWorldID::IBridgedWorldIDInstance;

pub(crate) trait Relay {
    /// Subscribe to the stream of new Roots on L1.
    async fn subscribe_roots(&self, rx: Receiver<Field>) -> Result<()>;
}

pub enum Relayer<E: RelaySigner, S: RelaySigner> {
    Evm(EVMRelay<E>),
    Svm(SvmRelay<S>),
}

impl<E, S> Relay for Relayer<E, S>
where
    E: RelaySigner,
    S: RelaySigner,
{
    async fn subscribe_roots(&self, rx: Receiver<Field>) -> Result<()> {
        match self {
            Relayer::Evm(relay) => relay.subscribe_roots(rx).await,
            Relayer::Svm(_relay) => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EVMRelay<S>
where
    S: RelaySigner,
{
    pub signer: S,
    pub world_id_address: Address,
    pub provider: Url,
}

impl<S> EVMRelay<S>
where
    S: RelaySigner,
{
    pub fn new(signer: S, world_id_address: Address, provider: Url) -> Self {
        Self {
            signer,
            world_id_address,
            provider,
        }
    }
}

impl<S> Relay for EVMRelay<S>
where
    S: RelaySigner,
{
    async fn subscribe_roots(&self, mut rx: Receiver<Field>) -> Result<()> {
        let l2_provider = ProviderBuilder::new().on_http(self.provider.clone());
        let world_id_instance = Arc::new(IBridgedWorldIDInstance::new(
            self.world_id_address,
            l2_provider,
        ));
        while let Some(field) = rx.recv().await {
            let world_id = world_id_instance.clone();
            let latest = world_id.latestRoot().call().await?._0;
            if latest != field {
                self.signer.propagate_root().await?;
            }
        }
        Ok(())
    }
}

pub struct SvmRelay<S>(PhantomData<S>);
