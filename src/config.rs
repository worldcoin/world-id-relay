use std::path::Path;

use alloy::primitives::Address;
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::rpc::client::ClientBuilder;
use alloy::transports::http::Http;
use alloy::transports::layers::{RetryBackoffLayer, RetryBackoffService};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The network from which roots will be propagated
    pub canonical_network: NetworkConfig,
    /// The networks to which roots will be propagated
    #[serde(default)]
    pub bridged_networks: Vec<NetworkConfig>,
    /// The number of blocks in the past to start scanning for new root events
    #[serde(default = "default::start_scan")]
    pub start_scan: u64,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(rename = "type")]
    pub ty: NetworkType,
    pub name: String,
    pub address: Address,
    #[serde(default)]
    pub state_bridge_address: Address,
    #[serde(default)]
    pub world_id_address: Address,
    pub provider: ProviderConfig,
    pub wallet: WalletConfig,
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
    Mnemonic {
        mnemonic: String,
    },
    TxSitter {
        url: String,
        address: Address,
        gas_limit: Option<u64>,
    },
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
    pub fn provider(&self) -> RootProvider<RetryBackoffService<Http<Client>>> {
        let client = ClientBuilder::default()
            .layer(RetryBackoffLayer::new(
                self.max_rate_limit_retries,
                self.initial_backoff,
                self.compute_units_per_second,
            ))
            .http(self.rpc_endpoint.clone());
        ProviderBuilder::new().on_client(client)
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
        1
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
