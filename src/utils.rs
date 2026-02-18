use std::{future::Future, time::Duration};

use backon::Retryable;

pub struct RetryConfig {
    pub min_delay: Duration,
    pub max_delay: Duration,
    pub max_times: usize,
}

pub async fn retry<F, Fut, T>(
    f: F,
    config: &RetryConfig,
    retry_msg: &str,
    exhausted_msg: &str,
) -> Result<T, eyre::Report>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, eyre::Report>>,
{
    let retry_msg = retry_msg.to_owned();
    let exhausted_msg = exhausted_msg.to_owned();

    f.retry(
        backon::ExponentialBuilder::default()
            .with_min_delay(config.min_delay)
            .with_max_delay(config.max_delay)
            .with_max_times(config.max_times),
    )
    .notify(move |e: &eyre::Report, dur| {
        tracing::warn!(error = ?e, retry_in = ?dur, "{}", retry_msg);
    })
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "{}", exhausted_msg);
        e
    })
}
