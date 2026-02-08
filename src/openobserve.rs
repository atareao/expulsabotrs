use serde_json::Value;
use reqwest::{Client, Response, StatusCode};
use reqwest::header::{HeaderMap, HeaderValue, HeaderName};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct OpenObserve{
    url: String,
    token: String,
}

#[derive(Debug)]
pub enum CustomError{
    ReqwestError(reqwest::Error),
    MessageError(String)
}


impl OpenObserve{
    pub fn new(base_url: &str, indice: &str, token: &str) -> Self{
        Self {
            url: format!("https://{}/api/default/{}/_json", base_url, indice),
            token: token.to_string(),
        }
    }

    async fn post(&self, url: &str, body: &Value)->Result<Response, CustomError>{
        println!("URL: {}", url);
        let mut header_map = HeaderMap::new();
        header_map.insert(HeaderName::from_str("Content-type").unwrap(),
                          HeaderValue::from_str("application/json").unwrap());
        header_map.insert(HeaderName::from_str("Accept").unwrap(),
                          HeaderValue::from_str("application/json").unwrap());
        header_map.insert(HeaderName::from_str("Authorization").unwrap(),
                          HeaderValue::from_str(&format!("Basic {}", self.token)).unwrap());
        let client = Client::builder()
            .default_headers(header_map)
            .build()
            .unwrap();
        let content = serde_json::to_string(body).unwrap();
        match client.post(url).body(content).send().await{
            Ok(res) => {
                if res.status() == StatusCode::OK{
                    Ok(res)
                }else{
                    let msg = "Esto es un error".to_string();
                    Err(CustomError::MessageError(msg))
                }

            },
            Err(e) => {
                Err(CustomError::ReqwestError(e))
            },
        }
    }
}
