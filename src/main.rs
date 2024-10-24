pub mod abi;
pub mod block_scanner;
pub mod config;
pub mod relay;
pub mod tx_sitter;
pub mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use alloy::network::EthereumWallet;
use alloy::primitives::U256;
use alloy::providers::Provider;
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
use tracing::info;
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

    // Set default log level if not set
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
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

    // // Start in the past by approximately 2 hours
    // let start_block_number = latest_block_number
    //     .checked_sub(config.canonical_network.start_scan)
    //     .unwrap_or_default();

    let filter = Filter::new()
        .address(config.canonical_network.world_id_addr)
        .event_signature(TreeChanged::SIGNATURE_HASH);

    let scanner = BlockScanner::new(
        provider.clone(),
        config.canonical_network.provider.window_size,
        latest_block_number,
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
            relay.subscribe_roots(tx.subscribe()).await.map_err(|error| {
                match relay {
                    Relayer::EVMRelay(EVMRelay {
                        signer: _,
                        world_id_address,
                        provider,
                    }) => {
                        tracing::error!(
                            %error,
                            %provider,
                            %world_id_address,
                            "Error subscribing to roots"
                        );
                    }
                    Relayer::SvmRelay(_) => {
                        tracing::error!(%error, "Error subscribing to roots");
                    }
                }
                eyre!(error)
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

/// Initializes the relayers for the bridged networks.
///
/// Additionally initializes the signers from the global wallet configuration if present,
/// otherwise from the bridged network configuration.
fn init_relays(cfg: Config) -> Result<Vec<Relayer>> {
    // Optinally use a global wallet configuration for all networks without a specific wallet configuration.
    let global_signer = if let Some(wallet) = cfg.canonical_network.wallet {
        match wallet {
            WalletConfig::Mnemonic { mnemonic } => {
                let signer = MnemonicBuilder::<English>::default()
                    .phrase(mnemonic)
                    .index(0)?
                    .build()?;
                let wallet = EthereumWallet::new(signer);

                let provider =
                    cfg.canonical_network.provider.signer(wallet.clone());

                Some(Arc::new(provider))
            }
            _ => None,
        }
    } else {
        None
    };
    cfg.bridged_networks
        .iter()
        .map(|bridged| {
            let wallet_config = bridged.wallet.as_ref();
            match bridged.ty {
                NetworkType::Evm => match wallet_config {
                    Some(WalletConfig::Mnemonic { mnemonic }) => {
                        let signer = MnemonicBuilder::<English>::default()
                            .phrase(mnemonic)
                            .index(0)?
                            .build()?;
                        let wallet = EthereumWallet::new(signer);
                        let provider = cfg
                            .canonical_network
                            .provider
                            .signer(wallet.clone());
                        let alloy_signer = AlloySigner::new(
                            bridged.state_bridge_addr,
                            Arc::new(provider),
                        );

                        Ok(Relayer::EVMRelay(EVMRelay::new(
                            Signer::AlloySigner(alloy_signer),
                            bridged.world_id_addr,
                            bridged.provider.rpc_endpoint.clone(),
                        )))
                    }
                    Some(WalletConfig::TxSitter { url, gas_limit }) => {
                        let signer = TxSitterSigner::new(
                            url.as_str(),
                            bridged.state_bridge_addr,
                            *gas_limit,
                        );

                        Ok(Relayer::EVMRelay(EVMRelay::new(
                            Signer::TxSitterSigner(signer),
                            bridged.world_id_addr,
                            bridged.provider.rpc_endpoint.clone(),
                        )))
                    }
                    None => {
                        if let Some(global_signer) = &global_signer {
                            info!(network = %bridged.name, "Using global wallet configuration for bridged network");
                            let alloy_signer = AlloySigner::new(
                                bridged.state_bridge_addr,
                                global_signer.clone(),
                            );

                            Ok(Relayer::EVMRelay(EVMRelay::new(
                                Signer::AlloySigner(alloy_signer),
                                bridged.world_id_addr,
                                bridged.provider.rpc_endpoint.clone(),
                            )))
                        } else {
                            Err(eyre!("No wallet configuration found"))
                        }
                    }
                },
                NetworkType::Svm => unimplemented!(),
                NetworkType::Scroll => unimplemented!(),
            }
        })
        .collect()
}
