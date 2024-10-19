use std::marker::PhantomData;
use std::sync::Arc;

use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::transports::Transport;
use eyre::Result;
use semaphore::Field;
use tokio::sync::mpsc::Receiver;

use crate::abi::IBridgedWorldID::IBridgedWorldIDInstance;

pub trait RelaySigner {
    /// Propogate a new Root to the State Bridge for the given network.
    async fn propagate_root(&self, root: Field) -> Result<()>;
}

pub(crate) trait Relay {
    /// Subscribe to the stream of new Roots on L1.
    async fn subscribe_roots(&'static self, rx: Receiver<Field>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct OptimismRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub signer: S,
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P, S> OptimismRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub async fn new(
        signer: S,
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            signer,
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

impl<T, P, S> Relay for OptimismRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
    S: RelaySigner,
{
    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.signer.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}

pub struct BaseRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub signer: S,
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P, S> BaseRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub async fn new(
        signer: S,
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            signer,
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

impl<T, P, S> Relay for BaseRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
    S: RelaySigner,
{
    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.signer.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorldChainRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub signer: S,
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P, S> WorldChainRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub async fn new(
        signer: S,
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            signer,
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

impl<T, P, S> Relay for WorldChainRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
    S: RelaySigner,
{
    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.signer.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PolygonRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub signer: S,
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P, S> PolygonRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T>,
    S: RelaySigner,
{
    pub async fn new(
        signer: S,
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            signer,
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

impl<T, P, S> Relay for PolygonRelayer<T, P, S>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
    S: RelaySigner,
{
    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.signer.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}
