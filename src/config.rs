use core::fmt;
use std::path::Path;

use alloy::network::EthereumWallet;
use alloy::primitives::Address;
use alloy::providers::fillers::{
    BlobGasFiller, CachedNonceManager, ChainIdFiller, GasFiller, NonceFiller,
};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::client::ClientBuilder;
use alloy::transports::http::Http;
use alloy::transports::layers::{RetryBackoffLayer, RetryBackoffService};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::relay::signer::AlloySignerProvider;

pub type ThrottledTransport = RetryBackoffService<Http<Client>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The network from which roots will be propagated
    pub canonical_network: CanonicalNetworkConfig,
    /// The networks to which roots will be propagated
    #[serde(default)]
    pub bridged_networks: Vec<BridgedNetworkConfig>,
    #[serde(default)]
    pub telemetry: Option<TelemetryConfig>,
}

impl Config {
    pub fn load(config_path: Option<&Path>) -> eyre::Result<Self> {
        let mut settings = config::Config::builder();

        if let Some(path) = config_path {
            settings =
                settings.add_source(config::File::from(path).required(true));
        }

        let settings = settings
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let config = serde_path_to_error::deserialize(settings)?;

        Ok(config)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BridgedNetworkConfig {
    /// The wallet configuration for the network
    /// overrides the global wallet configuration
    pub wallet: Option<WalletConfig>,
    pub state_bridge_addr: Address,
    pub world_id_addr: Address,
    #[serde(rename = "type")]
    pub ty: NetworkType,
    pub name: String,
    pub provider: ProviderConfig,
}

impl fmt::Debug for BridgedNetworkConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgedNetworkConfig")
            .field("state_bridge_addr", &self.state_bridge_addr)
            .field("world_id_addr", &self.world_id_addr)
            .field("ty", &self.ty)
            .field("name", &self.name)
            .field("provider", &self.provider)
            .finish()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanonicalNetworkConfig {
    pub world_id_addr: Address,
    /// The global wallet configuration
    pub wallet: Option<WalletConfig>,
    /// The number of blocks in the past to start scanning for new root events
    #[serde(default = "default::start_scan")]
    pub start_scan: u64,
    #[serde(rename = "type")]
    pub ty: NetworkType,
    pub name: String,
    pub provider: ProviderConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkType {
    Evm,
    Svm,
    Scroll,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WalletConfig {
    Mnemonic { mnemonic: String },
    TxSitter { url: String, gas_limit: Option<u64> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Ethereum RPC endpoint
    pub rpc_endpoint: Url,
    /// The maximum number of retries for rate limit errors
    #[serde(default = "default::max_rate_limit_retries")]
    pub max_rate_limit_retries: u32,
    /// The initial backoff in milliseconds
    #[serde(default = "default::initial_backoff")]
    pub initial_backoff: u64,
    /// The number of compute units per second for this provider
    #[serde(default = "default::compute_units_per_second")]
    pub compute_units_per_second: u64,
    #[serde(default = "default::window_size")]
    pub window_size: u64,
}

impl ProviderConfig {
    pub fn provider(&self) -> impl Provider<ThrottledTransport> {
        let client = ClientBuilder::default()
            .layer(RetryBackoffLayer::new(
                self.max_rate_limit_retries,
                self.initial_backoff,
                self.compute_units_per_second,
            ))
            .http(self.rpc_endpoint.clone());
        ProviderBuilder::new().on_client(client)
    }

    pub fn signer(&self, wallet: EthereumWallet) -> AlloySignerProvider {
        let client = ClientBuilder::default()
            .layer(RetryBackoffLayer::new(
                self.max_rate_limit_retries,
                self.initial_backoff,
                self.compute_units_per_second,
            ))
            .http(self.rpc_endpoint.clone());
        ProviderBuilder::new()
            .filler(ChainIdFiller::default())
            .filler(NonceFiller::new(CachedNonceManager::default()))
            .filler(BlobGasFiller)
            .filler(GasFiller::default())
            .wallet(wallet)
            .on_client(client)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    // Service name - used for logging, metrics and tracing
    pub service_name: String,
    // Traces
    pub traces_endpoint: Option<String>,
    // Metrics
    pub metrics: Option<MetricsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub host: String,
    pub port: u16,
    pub queue_size: usize,
    pub buffer_size: usize,
    pub prefix: String,
}

mod default {

    pub const fn window_size() -> u64 {
        1000
    }

    pub const fn max_rate_limit_retries() -> u32 {
        10
    }

    pub const fn initial_backoff() -> u64 {
        100
    }

    pub const fn compute_units_per_second() -> u64 {
        10000
    }

    pub const fn start_scan() -> u64 {
        600
    }
}
