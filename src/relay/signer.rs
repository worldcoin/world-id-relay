use alloy::{providers::Provider, transports::Transport};
use eyre::eyre::Result;
use tracing::{error, info};

use crate::abi::IStateBridge::IStateBridgeInstance;

pub trait RelaySigner {
    /// Propogate a new Root to the State Bridge for the given network.
    async fn propagate_root(&self) -> Result<()>;
}

pub struct AlloySigner<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub(crate) state_bridge_instance: IStateBridgeInstance<T, P>,
    pub chain_id: u64,
}

impl<T, P> AlloySigner<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    pub fn new(
        state_bridge: IStateBridgeInstance<T, P>,
        chain_id: u64,
    ) -> Self {
        Self {
            state_bridge_instance: state_bridge,
            chain_id,
        }
    }
}

impl<T, P> RelaySigner for AlloySigner<T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    async fn propagate_root(&self) -> Result<()> {
        let transport =
            self.state_bridge_instance.propogateRoot().send().await?;

        match transport.get_receipt().await {
            Ok(receipt) => {
                info!(receipt = ?receipt, chain = ?self.chain_id, "Successfully propogated Root to State Bridge.");
            }
            Err(e) => {
                error!(error = ?e, chain = ?self.chain_id, "Failed to propogate Root to State Bridge.");
            }
        }

        Ok(())
    }
}
