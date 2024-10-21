use std::time::Duration;

use eyre::eyre::{bail, eyre};
use tx_sitter_client::data::TxStatus;
use tx_sitter_client::TxSitterClient;

const MAX_ATTEMPTS: usize = 100;
const INTERVAL: Duration = Duration::from_secs(5);

/// Monitor a tx sitter transaction until it is mined
///
/// Will make [`MAX_ATTEMPTS`] attempts to get the transaction status from the tx sitter
/// in intervals of [`INTERVAL`] seconds.
pub async fn monitor_tx(
    client: &TxSitterClient,
    tx_id: &str,
) -> eyre::Result<()> {
    tracing::info!(tx_id, "monitoring transaction");
    let mut interval = tokio::time::interval(INTERVAL);
    // First tick is immediate
    interval.tick().await;

    for _ in 0..MAX_ATTEMPTS {
        interval.tick().await;
        let tx = client.get_tx(tx_id).await.map_err(|e| eyre!(e))?;

        match tx.status {
            Some(TxStatus::Mined) | Some(TxStatus::Finalized) => {
                tracing::info!(tx_id, "tx mined");
                return Ok(());
            }
            _ => {
                tracing::trace!(tx_id, "tx not yet mined");
            }
        }
    }

    bail!("monitor_tx timed out");
}
