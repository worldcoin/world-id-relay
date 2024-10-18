use std::{marker::PhantomData, sync::Arc};

use alloy::{primitives::Address, providers::Provider, transports::Transport};
use axum::async_trait;
use eyre::Result;
use semaphore::Field;
use tokio::sync::mpsc::Receiver;

use crate::abi::IBridgedWorldID::IBridgedWorldIDInstance;

#[async_trait]
pub trait Relay {
    /// Propogate a new Root to the State Bridge for the given network.
    async fn propagate_root(&self, root: Field) -> Result<()>;

    /// Subscribe to the stream of new Roots on L1.
    async fn subscribe_roots(&'static self, rx: Receiver<Field>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct OptimismRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P> OptimismRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub async fn new(
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P> Relay for OptimismRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
{
    async fn propagate_root(&self, root: Field) -> Result<()> {
        todo!()
    }

    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BaseRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P> BaseRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub async fn new(
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P> Relay for BaseRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
{
    async fn propagate_root(&self, root: Field) -> Result<()> {
        todo!()
    }

    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorldChainRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P> WorldChainRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub async fn new(
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P> Relay for WorldChainRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
{
    async fn propagate_root(&self, root: Field) -> Result<()> {
        todo!()
    }

    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PolygonRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub state_bridge_address: Address,
    pub op_world_id: Arc<IBridgedWorldIDInstance<T, P>>,
    _pd: PhantomData<T>,
}

impl<T, P> PolygonRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub async fn new(
        state_bridge_address: Address,
        op_world_id: IBridgedWorldIDInstance<T, P>,
    ) -> Self {
        Self {
            state_bridge_address,
            op_world_id: Arc::new(op_world_id),
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P> Relay for PolygonRelayer<T, P>
where
    T: Transport + Clone,
    P: Provider<T> + 'static,
{
    async fn propagate_root(&self, root: Field) -> Result<()> {
        todo!()
    }

    async fn subscribe_roots(
        &'static self,
        mut rx: Receiver<Field>,
    ) -> Result<()> {
        while let Some(field) = rx.recv().await {
            let latest = self.op_world_id.latestRoot().await?._0;
            if latest != field {
                self.propagate_root(field).await?;
            }
        }
        Ok(())
    }
}
