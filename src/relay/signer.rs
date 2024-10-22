use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{bytes, Address, Bytes};
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill,
    NonceFiller, WalletFiller,
};
use alloy::providers::{Identity, RootProvider};
use alloy::transports::http::Http;
use ethers_core::types::U256;
use eyre::eyre::{eyre, Result};
use tracing::{error, info};
use tx_sitter_client::data::{SendTxRequest, TransactionPriority, TxStatus};
use tx_sitter_client::TxSitterClient;

use crate::abi::IStateBridge::IStateBridgeInstance;

/// "propogateRoot()" Selector
pub static PROPOGATE_ROOT_SELECTOR: Bytes = bytes!("21823a11");

pub(crate) trait RelaySigner {
    /// Propogate a new Root to the State Bridge for the given network.
    async fn propagate_root(&self) -> Result<()>;
}

macro_rules! signer {
    ($($signer_type:ident),+ $(,)?) => {
        pub enum Signer {
            $($signer_type($signer_type),)+
        }
        impl RelaySigner for Signer {
            async fn propagate_root(&self) -> Result<()> {
                match self {
                    $(Signer::$signer_type(signer) => signer.propagate_root().await,)+
                }
            }
        }
    }
}

pub type AlloySignerProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
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

pub struct TxSitterSigner {
    tx_sitter: TxSitterClient,
    state_bridge_address: Address,
    gas_limit: Option<u64>,
}

impl TxSitterSigner {
    pub fn new(
        url: &str,
        state_bridge_address: Address,
        gas_limit: Option<u64>,
    ) -> Self {
        let tx_sitter = TxSitterClient::new(url);
        Self {
            tx_sitter,
            state_bridge_address,
            gas_limit,
        }
    }
}

impl RelaySigner for TxSitterSigner {
    /// Propogate a new Root to the given network.
    ///
    /// This is a long running operation and should probably be awaited in a background task.
    async fn propagate_root(&self) -> Result<()> {
        let ethers_selector = ethers_core::types::Bytes::from_static(
            PROPOGATE_ROOT_SELECTOR.as_ref(),
        );
        let ethers_address = ethers_core::types::Address::from_slice(
            self.state_bridge_address.as_ref(),
        );
        let send_tx = SendTxRequest {
            to: ethers_address,
            data: Some(ethers_selector),
            gas_limit: self.gas_limit.map(U256::from).unwrap_or_default(),
            priority: TransactionPriority::Fast,
            value: U256::zero(),
            tx_id: None,
        };

        let resp = self.tx_sitter.send_tx(&send_tx).await.map_err(|e| {
            eyre!(
                "Failed to send root propogation transaction to tx sitter: {}",
                e
            )
        })?;

        info!(
            tx_id = &resp.tx_id,
            "Successfully sent root propogation transaction to tx sitter"
        );
        let timeout = std::time::Duration::from_secs(120); // TODO: Should be configurable?
        let backoff = std::time::Duration::from_secs(12);
        let start = std::time::Instant::now();
        loop {
            let tx_response =
                self.tx_sitter.get_tx(&resp.tx_id).await.map_err(|e| {
                    eyre!("Failed to get tx status from tx sitter: {}", e)
                })?;

            match tx_response.status {
                Some(TxStatus::Mined) | Some(TxStatus::Finalized) => {
                    info!(
                        tx_id = &resp.tx_id,
                        "Root propogation transaction mined"
                    );
                    break;
                }
                _ => {
                    info!(
                        tx_id = &resp.tx_id,
                        "Root propogation transaction not yet mined"
                    );
                }
            }

            if start.elapsed() > timeout {
                return Err(eyre!("Root propogation transaction timed out"));
            }

            std::thread::sleep(backoff);
        }

        Ok(())
    }
}

signer!(AlloySigner, TxSitterSigner);
