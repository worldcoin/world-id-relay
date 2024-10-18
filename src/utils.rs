use std::future::Future;
use std::str::FromStr;
use std::time::Duration;

use ethers::types::H160;
use serde::{de, Deserialize, Deserializer};
use tracing::{error, warn};

pub async fn retry<S, F, T, E>(
    mut backoff: Duration,
    limit: Option<Duration>,
    f: S,
) -> Result<T, E>
where
    F: Future<Output = Result<T, E>> + Send + 'static,
    S: Fn() -> F + Send + Sync + 'static,
    E: std::fmt::Debug,
{
    loop {
        match f().await {
            Ok(res) => return Ok(res),
            Err(e) => {
                warn!("{e:?}");
                if let Some(limit) = limit {
                    if backoff > limit {
                        error!("Retry limit reached: {e:?}");
                        return Err(e);
                    }
                }
                tokio::time::sleep(backoff).await;
                backoff *= 2;
            }
        }
    }
}

pub fn deserialize_h160<'de, D>(deserializer: D) -> Result<H160, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    H160::from_str(&s).map_err(|e| {
        de::Error::custom(format!("Failed to deserialize H160: {}", e))
    })
}

pub fn deserialize_optional_h160<'de, D>(
    deserializer: D,
) -> Result<Option<H160>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(str_val) => H160::from_str(&str_val).map(Some).map_err(|e| {
            de::Error::custom(format!("Failed to deserialize H160: {}", e))
        }),
        None => Ok(None),
    }
}
