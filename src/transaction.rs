use std::sync::Arc;

use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Bytes, TransactionRequest, H160, H256, U256};
use eyre::{self};
use tokio::time::Duration;

use crate::abis::IStateBridge;

pub const FIVE_SECONDS: Duration = Duration::from_secs(5);

pub async fn construct_state_bridge_tx<M: 'static + Middleware>(
    to: H160,
    middleware: Arc<M>,
) -> eyre::Result<TypedTransaction> {
    let calldata = IStateBridge::new(H160::zero(), middleware.clone())
        .propagate_root()
        .calldata()
        .expect("Could not get calldata for propagateRoot()");

    let tx =
        fill_and_simulate_legacy_transaction(calldata, to, middleware).await?;

    Ok(tx)
}

async fn fill_and_simulate_legacy_transaction<M: 'static + Middleware>(
    calldata: Bytes,
    to: H160,
    middleware: Arc<M>,
) -> eyre::Result<TypedTransaction> {
    let gas_price = middleware.get_gas_price().await?;
    let tx = TransactionRequest::new()
        .to(to)
        .data(calldata)
        .gas_price(gas_price);
    let mut tx: TypedTransaction = tx.into();
    let gas_limit = middleware.estimate_gas(&tx, None).await?;

    tx.set_gas(gas_limit * U256::from(120) / U256::from(100)); // 20% buffer

    //match fill transaction, it will fail if the calldata fails
    middleware.fill_transaction(&mut tx, None).await?;
    middleware.call(&tx, None).await?;

    Ok(tx)
}

pub async fn wait_for_tx_receipt<M: 'static + Middleware>(
    pending_tx: H256,
    mut timeout: Duration,
    middleware: Arc<M>,
) -> eyre::Result<()> {
    while middleware
        .get_transaction_receipt(pending_tx)
        .await?
        .is_none()
    {
        tokio::time::sleep(FIVE_SECONDS).await;
        timeout -= FIVE_SECONDS;

        if timeout.is_zero() {
            tracing::error!("Tx timed out: {pending_tx}");
            return Err(eyre::eyre!("Pending tx timed out: {pending_tx}"));
        }
    }

    tracing::info!("Tx confirmed: {pending_tx:?}");

    Ok(())
}
