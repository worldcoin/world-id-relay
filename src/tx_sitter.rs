use std::time::Duration;

use eyre::eyre::{bail, eyre};
use tx_sitter_client::data::TxStatus;
use tx_sitter_client::TxSitterClient;

const MAX_ATTEMPTS: usize = 100;
const ATTEMPT_SLEEP: Duration = Duration::from_secs(5);

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
    client: &TxSitterClient,
    tx_id: &str,
) -> eyre::Result<()> {
    let mut num_attempts = 0;
    tracing::info!("Getting tx for id {}", tx_id);
    loop {
        let tx = client.get_tx(tx_id).await.map_err(|e| eyre!(e))?;

        match tx.status {
            Some(TxStatus::Mined) | Some(TxStatus::Finalized) => {
                tracing::info!("Got transaction from tx-sitter");
                return Ok(());
            }
            _ => {
                num_attempts += 1;
                if num_attempts >= MAX_ATTEMPTS {
                    bail!("Wasn't able to get transaction hash");
                } else {
                    tracing::warn!(
                        "Waiting for transaction to be broadcast, retrying"
                    );
                    tokio::time::sleep(ATTEMPT_SLEEP).await;
                }
            }
        }
    }
}
