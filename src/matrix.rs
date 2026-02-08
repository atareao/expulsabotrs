use pulldown_cmark::{html::push_html, Parser};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};
use urlencoding::encode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Matrix {
    url: String,
    token: String,
    room: String,
}

#[derive(Debug)]
pub enum CustomError {
    ReqwestError(reqwest::Error),
    MessageError(String),
}

impl Matrix {
    pub fn new(url: &str, token: &str, room: &str) -> Self {
        Self {
            url: url.to_string(),
            token: token.to_string(),
            room: room.to_string(),
        }
    }

    pub async fn post(&self, room: &str, markdown: &str) -> Result<Response, CustomError> {
        info!("post_with_matrix");
        let url = format!(
            "https://{}/_matrix/client/v3/rooms/{}:{}/send/m.room.message/{}",
            self.url,
            encode(room),
            self.url,
            Self::ts(),
        );
        debug!("Url: {}", url);
        let parser = Parser::new(&markdown);
        let mut html = String::new();
        push_html(&mut html, parser);
        debug!("Post with matrix: {}\n{}", markdown, html);
        let body = json!({
            "msgtype": "m.text",
            "body": markdown,
            "format": "org.matrix.custom.html",
            "formatted_body": html,
        });
        debug!("Body: {}", body);
        let mut header_map = HeaderMap::new();
        header_map.insert(
            HeaderName::from_str("Content-type").unwrap(),
            HeaderValue::from_str("application/json").unwrap(),
        );
        header_map.append(
            HeaderName::from_str("Authorization").unwrap(),
            HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap(),
        );
        debug!("Header: {:?}", header_map);
        Self::_put(&url, header_map, &body).await
    }

    pub async fn send_message(&self, message: &str) -> Result<(), CustomError> {
        let url = format!(
            "https://{}/_matrix/client/v3/rooms/{}:{}/send/m.room.message/{}",
            self.url,
            encode(&self.room),
            self.url,
            Self::ts(),
        );
        let body = json!({
            "msgtype": "m.text",
            "body": message,
        });
        let mut header_map = HeaderMap::new();
        header_map.insert(
            HeaderName::from_str("Content-type").unwrap(),
            HeaderValue::from_str("application/json").unwrap(),
        );
        header_map.append(
            HeaderName::from_str("Authorization").unwrap(),
            HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap(),
        );
        match Self::_put(&url, header_map, &body).await {
            Ok(_) => {
                debug!("Message sent successfully to Matrix");
                Ok(())
            }
            Err(e) => {
                debug!("Failed to send message to Matrix: {:?}", e);
                Err(e)
            }
        }
    }

    async fn _put(url: &str, header_map: HeaderMap, body: &Value) -> Result<Response, CustomError> {
        let client = Client::builder()
            .default_headers(header_map)
            .build()
            .unwrap();
        let content = serde_json::to_string(body).unwrap();
        match client.put(url).body(content).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    Ok(res)
                } else {
                    Err(CustomError::MessageError(format!(
                        "Failed to send message: HTTP {}",
                        res.status()
                    )))
                }
            }
            Err(e) => Err(CustomError::ReqwestError(e)),
        }
    }

    fn ts() -> f64 {
        debug!("ts");
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}
