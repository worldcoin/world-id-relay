use eyre::Result;
use tx_sitter_client::rpc::TxSitterRpcClient;
use tx_sitter_client::{TxSitterClient, TxSitterConfig};

use crate::config::NetworkConfig;

// Create an init function that saves an HashMap in a OneCell storing a string value for each Enum Network value
pub async fn init_tx_sitter(
    network: &NetworkConfig,
) -> Result<(String, TxSitterClient, TxSitterRpcClient)> {
    let tx_sitter_client = TxSitterClient::new(TxSitterConfig {
        send_lambda_name: network.send_lambda_name.clone(),
        rpc_lambda_name: network.rpc_lambda_name.clone(),
        transactions_lambda_name: network.transactions_lambda_name.clone(),
    })
    .await;

    let provider = tx_sitter_client.get_provider(&network.relayer_id);

    Ok((network.relayer_id.clone(), tx_sitter_client, provider))
}
