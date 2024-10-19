pub mod abi;
pub mod block_scanner;
pub mod config;
pub mod relay;
pub mod tx_sitter;
pub mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use alloy::providers::Provider as _;
use alloy::rpc::types::Filter;
use alloy::sol_types::SolEvent;
use clap::Parser;
use eyre::eyre::Result;
use futures::StreamExt;
use telemetry_batteries::metrics::statsd::StatsdBattery;
use telemetry_batteries::tracing::datadog::DatadogBattery;
use telemetry_batteries::tracing::TracingShutdownHandle;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use self::abi::IWorldIDIdentityManager::TreeChanged;
use self::block_scanner::BlockScanner;
use self::config::Config;

/// This service syncs the state of the World Tree and spawns a server that can deliver inclusion proofs for a given identity.
#[derive(Parser, Debug)]
#[clap(name = "World Id Relay")]
#[clap(version)]
struct Opts {
    /// Path to the configuration file
    #[clap(short, long)]
    config: Option<PathBuf>,

    /// Set to disable colors in the logs
    #[clap(long)]
    no_ansi: bool,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    eyre::install()?;
    dotenv::dotenv().ok();

    let opts = Opts::parse();

    let config = Config::load(opts.config.as_deref())?;

    let _tracing_shutdown_handle = if let Some(telemetry) = &config.telemetry {
        let tracing_shutdown_handle = DatadogBattery::init(
            telemetry.traces_endpoint.as_deref(),
            &telemetry.service_name,
            None,
            true,
        );

        if let Some(metrics_config) = &telemetry.metrics {
            StatsdBattery::init(
                &metrics_config.host,
                metrics_config.port,
                metrics_config.queue_size,
                metrics_config.buffer_size,
                Some(&metrics_config.prefix),
            )?;
        }

        tracing_shutdown_handle
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(!opts.no_ansi)
                    .pretty()
                    .compact(),
            )
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        TracingShutdownHandle
    };

    tracing::info!(?config, "Starting World Tree service");

    run(config).await
}

pub async fn run(config: Config) -> Result<()> {
    let provider = Arc::new(config.canonical_network.provider.provider());
    let chain_id = provider.get_chain_id().await?;

    let latest_block_number = provider.get_block_number().await?;

    // Start in the past by approximately 1 hour
    // TODO: Make this configurable
    let start_block_number =
        latest_block_number.checked_sub(300).unwrap_or_default();

    let filter = Filter::new()
        .address(config.canonical_network.address)
        .event_signature(TreeChanged::SIGNATURE_HASH);

    let scanner = BlockScanner::new(
        provider.clone(),
        config.canonical_network.provider.window_size,
        start_block_number,
        filter,
    )
    .await?;

    tracing::info!(chain_id, latest_block_number, "Starting ingestion");
    scanner
        .root_stream()
        .for_each(|x| async move {
            println!("{:#?}", x);
        })
        .await;

    Ok(())
}
