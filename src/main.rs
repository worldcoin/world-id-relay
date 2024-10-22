pub mod abi;
pub mod block_scanner;
pub mod config;
pub mod relay;
pub mod tx_sitter;
pub mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use abi::IStateBridge::IStateBridgeInstance;
use alloy::network::EthereumWallet;
use alloy::primitives::U256;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Filter;
use alloy::signers::local::MnemonicBuilder;
use alloy::sol_types::SolEvent;
use alloy_signer_local::coins_bip39::English;
use clap::Parser;
use config::{NetworkType, WalletConfig};
use eyre::eyre::{eyre, Result};
use futures::StreamExt;
use relay::signer::{AlloySigner, Signer, TxSitterSigner};
use relay::{EVMRelay, Relay, Relayer};
use telemetry_batteries::metrics::statsd::StatsdBattery;
use telemetry_batteries::tracing::datadog::DatadogBattery;
use telemetry_batteries::tracing::TracingShutdownHandle;
use tokio::task::JoinSet;
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

    // Start in the past by approximately 2 hours
    let start_block_number = latest_block_number
        .checked_sub(config.start_scan)
        .unwrap_or_default();

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

    let (tx, _) = tokio::sync::broadcast::channel::<U256>(1000);
    let relayers = init_relays(config)?;
    let mut joinset = JoinSet::new();
    for relay in relayers {
        let tx = tx.clone();
        joinset.spawn(async move {
            relay.subscribe_roots(tx.subscribe()).await.map_err(|e| {
                tracing::error!(?e, "Error subscribing to roots");
                eyre!(e)
            })?;
            Ok::<(), eyre::Report>(())
        });
    }

    let scanner_fut = async {
        scanner
            .root_stream()
            .for_each(|event| {
                let tx = tx.clone();
                async move {
                    let field = event.postRoot;
                    if let Err(e) = tx.send(field) {
                        tracing::error!(?e, "Error sending root");
                    }
                }
            })
            .await;
    };

    tokio::select! {
        _ = scanner_fut => {
            tracing::error!("Scanner task failed");
        }
        _ = joinset.join_all() => {
            tracing::error!("Relayer task failed");
        }
    }
    Ok(())
}

fn init_relays(cfg: Config) -> Result<Vec<Relayer>> {
    let mut relayers = Vec::new();
    cfg.bridged_networks.iter().for_each(|n| match n.ty {
        NetworkType::Evm => {
            match &cfg.canonical_network.wallet {
                WalletConfig::Mnemonic { mnemonic } => {
                    let signer = MnemonicBuilder::<English>::default()
                        .phrase(mnemonic)
                        .index(0)
                        .unwrap()
                        .build()
                        .expect("Failed to build wallet");
                    let wallet = EthereumWallet::new(signer);
                    let l1_provider = ProviderBuilder::default()
                        .with_recommended_fillers()
                        .wallet(wallet)
                        .on_http(
                            cfg.canonical_network.provider.rpc_endpoint.clone(),
                        );
                    let state_bridge = IStateBridgeInstance::new(
                        n.state_bridge_address,
                        l1_provider,
                    );

                    let signer = AlloySigner::new(state_bridge);

                    relayers.push(Relayer::Evm(EVMRelay::new(
                        Signer::AlloySigner(signer),
                        n.world_id_address,
                        n.provider.rpc_endpoint.clone(),
                    )));
                }
                WalletConfig::TxSitter {
                    url,
                    address: _,
                    gas_limit,
                } => {
                    let signer = TxSitterSigner::new(
                        url.as_str(),
                        n.state_bridge_address,
                        *gas_limit,
                    );

                    relayers.push(Relayer::Evm(EVMRelay::new(
                        Signer::TxSitterSigner(signer),
                        n.world_id_address,
                        n.provider.rpc_endpoint.clone(),
                    )));
                }
            };
        }
        NetworkType::Svm => unimplemented!(),
        NetworkType::Scroll => unimplemented!(),
    });

    Ok(relayers)
}
