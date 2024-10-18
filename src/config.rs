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
    pub bridged_networks: Vec<NetworkConfig>,
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
    pub kind: NetworkKind,
    pub name: String,
    pub address: Address,
    pub provider: ProviderConfig,
    pub send_lambda_name: String,
    pub rpc_lambda_name: String,
    pub transactions_lambda_name: String,
    pub relayer_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkKind {
    Evm,
    Svm,
    Scroll,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Ethereum RPC endpoint
    #[serde(with = "serde_url")]
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

mod serde_url {
    use std::borrow::Cow;

    use serde::{Deserialize, Serializer};
    use url::Url;

    pub fn serialize<S>(url: &Url, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(url.as_ref())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: Cow<'static, str> = Deserialize::deserialize(deserializer)?;

        Url::parse(&s).map_err(serde::de::Error::custom)
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
}
