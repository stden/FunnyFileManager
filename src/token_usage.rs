//! Purpose: Read provider usage reports over bounded authenticated HTTP requests.
use crate::models::token_usage::{Provider, ProviderUsage, UsageError, UsageTotals};
mod codec;
mod http;
#[cfg(test)]
mod tests;
pub trait UsageHttp {
    fn get(
        &mut self,
        provider: Provider,
        page: Option<&str>,
    ) -> Result<serde_json::Value, UsageError>;
}
pub fn collect() -> Vec<ProviderUsage> {
    Provider::ALL
        .into_iter()
        .map(|provider| {
            let result =
                http::Client::new(provider).and_then(|mut client| report(&mut client, provider));
            ProviderUsage { provider, result }
        })
        .collect()
}
pub fn report(http: &mut dyn UsageHttp, provider: Provider) -> Result<UsageTotals, UsageError> {
    let mut total = UsageTotals::default();
    let mut cursor = None;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..100 {
        let body = http.get(provider, cursor.as_deref())?;
        let page = codec::parse(provider, &body)?;
        codec::merge(&mut total, &page.totals)?;
        match page.next {
            None => return Ok(total),
            Some(next) if next.len() <= 2048 && seen.insert(next.clone()) => cursor = Some(next),
            Some(_) => return Err(UsageError::Limit),
        }
    }
    Err(UsageError::Limit)
}
