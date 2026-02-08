use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct UserEvent {
    pub user_id: i64,
    pub user_name: String,
    pub group_id: i64,
    pub group_name: String,
    pub challenge_completed: bool,
    pub banned: bool,
}

#[derive(Debug, Clone)]
pub struct OpenObserve {
    url: String,
    token: String,
}

#[derive(Debug)]
pub enum CustomError {
    ReqwestError(reqwest::Error),
    MessageError(String),
}

impl OpenObserve {
    pub fn new(base_url: &str, indice: &str, token: &str) -> Self {
        Self {
            url: format!("https://{}/api/default/{}/_json", base_url, indice),
            token: token.to_string(),
        }
    }

    async fn post(&self, url: &str, body: &Value) -> Result<Response, CustomError> {
        println!("URL: {}", url);
        let mut header_map = HeaderMap::new();
        header_map.insert(
            HeaderName::from_str("Content-type").unwrap(),
            HeaderValue::from_str("application/json").unwrap(),
        );
        header_map.insert(
            HeaderName::from_str("Accept").unwrap(),
            HeaderValue::from_str("application/json").unwrap(),
        );
        header_map.insert(
            HeaderName::from_str("Authorization").unwrap(),
            HeaderValue::from_str(&format!("Basic {}", self.token)).unwrap(),
        );
        let client = Client::builder()
            .default_headers(header_map)
            .build()
            .unwrap();
        let content = serde_json::to_string(body).unwrap();
        match client.post(url).body(content).send().await {
            Ok(res) => {
                if res.status() == StatusCode::OK {
                    Ok(res)
                } else {
                    let msg = "Esto es un error".to_string();
                    Err(CustomError::MessageError(msg))
                }
            }
            Err(e) => Err(CustomError::ReqwestError(e)),
        }
    }

    pub async fn send_user_event(&self, event: &UserEvent) -> Result<(), CustomError> {
        let event_json = serde_json::to_value(event)
            .map_err(|e| CustomError::MessageError(format!("Failed to serialize event: {}", e)))?;

        match self.post(&self.url, &event_json).await {
            Ok(_) => {
                println!("User event sent successfully to OpenObserve");
                Ok(())
            }
            Err(e) => {
                println!("Failed to send user event to OpenObserve: {:?}", e);
                Err(e)
            }
        }
    }
}
