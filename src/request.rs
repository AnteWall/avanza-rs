use std::collections::HashMap;

use reqwest::Response;
use serde::de::DeserializeOwned;

use crate::error::RequestError;

pub async fn post_response<T: DeserializeOwned>(
    uri: &str,
    json_body: &HashMap<&str, &str>,
) -> Result<T, RequestError> {
    let http_client = reqwest::Client::new();
    let response = http_client.post(uri).json(json_body).send().await?;
    let body = response.text().await?;
    Ok(serde_json::from_str::<T>(&body)?)
}
pub async fn post(uri: &str, json_body: &HashMap<&str, &str>) -> Result<Response, RequestError> {
    let http_client = reqwest::Client::new();
    Ok(http_client.post(uri).json(json_body).send().await?)
}
