use std::convert::Into;
use std::str::FromStr;
use std::time::Duration;

use ethers::providers::{Middleware, Provider};
use ethers::types::{TransactionReceipt, H256};
use eyre::ContextCompat;
use tx_sitter_client::rpc::TxSitterRpcClient;
use tx_sitter_client::types::transaction::TxSitterTransaction;
use tx_sitter_client::TxSitterClient;

const MAX_ATTEMPTS: usize = 100;
const ATTEMPT_SLEEP: Duration = Duration::from_secs(5);

static ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

/// This function monitors a transaction on the Optimism chain.
/// It will wait until the transaction either fails or succeeds.
/// If the transaction fails (i.e. status != 1 on chain or status Failed in OZ Relayer), it will return an error.
/// If the transaction succeeds, it will return Ok(()).
///
/// This function tries to fetch the tx receipt in two ways at the same time:
/// 1. From the Optimism RPC
/// 2. From the OZ Relayer
///
/// It will return the first receipt it gets.
///
/// If at any point we encounter an error, we will retry after ATTEMPT_SLEEP until we reach MAX_ATTEMPTS.
/// at that point we will return an error.
pub async fn monitor_tx(
    tx_sitter_client: &TxSitterClient,
    provider: &Provider<TxSitterRpcClient>,
    tx_id: &str,
) -> eyre::Result<()> {
    let tx = get_tx(tx_sitter_client, tx_id).await?;
    let tx_hash = tx.clone().tx_hash;

    tracing::info!(tx_hash = ?tx_hash, "Checking tx receipt");

    let tx_receipt = get_receipt(provider, &tx_hash).await?;

    tracing::info!("Got tx receipt");

    if tx_receipt.status != Some(1.into()) {
        tracing::error!("Transaction failed on chain {}", tx_id);
        eyre::bail!("Transaction failed on chain");
    }

    Ok(())
}

async fn get_tx(
    tx_sitter_client: &TxSitterClient,
    tx_id: &str,
) -> eyre::Result<TxSitterTransaction> {
    let mut num_attempts = 0;

    tracing::info!("Getting tx for id {}", tx_id);

    loop {
        let tx: TxSitterTransaction = tx_sitter_client.get_transaction_by_id(tx_id).await?;

        // Transactions are initially created with zero address as tx_hash
        if tx.tx_hash == ZERO_ADDRESS {
            num_attempts += 1;
            if num_attempts >= MAX_ATTEMPTS {
                eyre::bail!("Wasn't able to get transaction hash");
            } else {
                tracing::warn!("Waiting for transaction to be broadcast, retrying");
                tokio::time::sleep(ATTEMPT_SLEEP).await;
            }
        } else {
            tracing::info!("Got transaction from tx-sitter");
            return Ok(tx);
        }
    }
}

async fn get_receipt(
    provider: &Provider<TxSitterRpcClient>,
    tx_hash: &str,
) -> eyre::Result<TransactionReceipt> {
    let mut num_attempts = 0;

    loop {
        let tx_receipt = provider
            .get_transaction_receipt(H256::from_str(tx_hash)?)
            .await?
            .context("Missing tx receipt")?;

        // Transactions hash is populated before it gets mined
        if tx_receipt.status.is_none() {
            num_attempts += 1;
            if num_attempts >= MAX_ATTEMPTS {
                eyre::bail!("Wasn't able to get transaction receipt");
            } else {
                tracing::warn!("Waiting for transaction to be mined, retrying");
                tokio::time::sleep(ATTEMPT_SLEEP).await;
            }
        } else {
            tracing::info!("Got transaction receipt from provider");
            return Ok(tx_receipt);
        }
    }
}
