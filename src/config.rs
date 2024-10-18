use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The network from which roots will be propagated
    pub canonical_network: NetworkConfig,
    /// The networks to which roots will be propagated
    pub bridged_trees: Vec<NetworkConfig>,
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
