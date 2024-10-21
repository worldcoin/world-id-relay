use alloy::network::{Ethereum, EthereumWallet};
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill,
    NonceFiller, WalletFiller,
};
use alloy::providers::RootProvider;
use alloy::transports::http::Http;
use eyre::eyre::Result;
use tracing::{error, info};

use crate::abi::IStateBridge::IStateBridgeInstance;

pub trait RelaySigner {
    #[allow(async_fn_in_trait)]
    /// Propogate a new Root to the State Bridge for the given network.
    async fn propagate_root(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Signer {
    Alloy(AlloySigner),
    TxSitter, // TODO: Implement this
}

impl RelaySigner for Signer {
    async fn propagate_root(&self) -> Result<()> {
        match self {
            Signer::Alloy(signer) => signer.propagate_root().await,
            Signer::TxSitter => unimplemented!(),
        }
    }
}

pub type AlloySignerProvider = FillProvider<
    JoinFill<
        JoinFill<
            alloy::providers::Identity,
            JoinFill<
                GasFiller,
                JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>,
            >,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Http<reqwest::Client>>,
    Http<reqwest::Client>,
    Ethereum,
>;
#[derive(Debug, Clone)]
pub struct AlloySigner {
    pub(crate) state_bridge_instance:
        IStateBridgeInstance<Http<reqwest::Client>, AlloySignerProvider>,
}

impl AlloySigner {
    pub fn new(
        state_bridge: IStateBridgeInstance<
            Http<reqwest::Client>,
            AlloySignerProvider,
        >,
    ) -> Self {
        Self {
            state_bridge_instance: state_bridge,
        }
    }
}

impl RelaySigner for AlloySigner {
    async fn propagate_root(&self) -> Result<()> {
        let transport =
            self.state_bridge_instance.propogateRoot().send().await?;

        match transport.get_receipt().await {
            Ok(receipt) => {
                info!(receipt = ?receipt, "Successfully propogated Root to State Bridge.");
            }
            Err(e) => {
                error!(error = ?e, "Failed to propogate Root to State Bridge.");
            }
        }

        Ok(())
    }
}
