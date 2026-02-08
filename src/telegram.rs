use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use tracing_subscriber::field::debug;

pub const TELEGRAM_API_URL: &str = "https://api.telegram.org/bot";

// --- Structs for Telegram API responses ---

#[derive(Debug, Deserialize, Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_premium: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")] // 'type' is a reserved keyword in Rust
    pub chat_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_forum: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Member {
    pub status: String, // e.g., "member", "administrator", "restricted", "left", "kicked"
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: u64,
    pub chat: Chat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub from: User,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_chat_members: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_chat_member: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_chat_participant: Option<User>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMemberUpdated {
    pub chat: Chat,
    pub from: User,
    pub date: u64,
    pub old_chat_member: Member,
    pub new_chat_member: Member,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub update_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_query: Option<CallbackQuery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_member: Option<ChatMemberUpdated>,
    // Add other possible update types that we're not using
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_message: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_post: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_channel_post: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_query: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chosen_inline_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_query: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_checkout_query: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_answer: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_chat_member: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_join_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SentMessageResult {
    pub message_id: u64,
    pub chat: Chat,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: T,
    pub description: Option<String>,
}

// --- Helper functions for Telegram API interactions ---

#[derive(Debug, Clone)]
pub struct Telegram {
    client: Client,
    token: String,
}

impl Telegram {
    pub fn new(token: &str) -> Self {
        Telegram {
            client: Client::new(),
            token: token.to_string(),
        }
    }

    pub async fn send_request<T: for<'de> Deserialize<'de> + 'static>(
        &self,
        method: &str,
        json_payload: serde_json::Value,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}{}/{}", TELEGRAM_API_URL, &self.token, method);
        debug!(
            "Sending request to Telegram API: {} with payload: {}",
            url, json_payload
        );
        let response = self.client.post(&url).json(&json_payload).send().await?;

        let response_json = response.json::<TelegramResponse<T>>().await?;

        if response_json.ok {
            Ok(response_json.result)
        } else {
            let error_message = response_json
                .description
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(format!(
                "Telegram API Error ({}/{}): {}",
                method, response_json.ok, error_message
            )
            .into())
        }
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML",
        });
        self.send_request("sendMessage", payload)
            .await
            .map(|result: SentMessageResult| result.message_id)
    }

    pub async fn send_message_with_keyboard(
        &self,
        chat_id: i64,
        text: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": serde_json::to_value(keyboard)?,
        });
        self.send_request("sendMessage", payload)
            .await
            .map(|result: SentMessageResult| result.message_id)
    }

    pub async fn restrict_chat_member(
        &self,
        chat_id: i64,
        user_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "user_id": user_id,
            "permissions": {
                "can_send_messages": false,
                "can_send_media_messages": false,
                "can_send_other_messages": false,
                "can_add_web_page_previews": false,
                "can_change_info": false,
                "can_invite_users": false,
                "can_pin_messages": false,
            },
            "use_independent_chat_permissions": false,
            "until_date": 0
        });
        let _: bool = self.send_request("restrictChatMember", payload).await?;
        Ok(())
    }

    pub async fn unrestrict_chat_member(
        &self,
        chat_id: i64,
        user_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "user_id": user_id,
            "permissions": {
                "can_send_messages": true,
                "can_send_media_messages": true,
                "can_send_other_messages": true,
                "can_add_web_page_previews": true,
                "can_change_info": true,
                "can_invite_users": true,
                "can_pin_messages": true,
            },
            "use_independent_chat_permissions": false,
            "until_date": 0
        });
        let _: bool = self.send_request("restrictChatMember", payload).await?;
        Ok(())
    }

    pub async fn ban_chat_member(
        &self,
        chat_id: i64,
        user_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "user_id": user_id,
            "revoke_messages": true
        });
        let _: bool = self.send_request("banChatMember", payload).await?;
        Ok(())
    }

    pub async fn delete_message(
        &self,
        chat_id: i64,
        message_id: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
        });
        let _: bool = self.send_request("deleteMessage", payload).await?;
        Ok(())
    }

    pub async fn get_updates(
        &self,
        offset: u64,
    ) -> Result<Vec<Update>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}{}/getUpdates?offset={}&timeout=60",
            TELEGRAM_API_URL, &self.token, offset
        );

        // Debug: log the URL being called
        debug!("Calling Telegram API: {}", url);

        let response = self.client.get(&url).send().await?;
        debug!("Received response from Telegram API: {:?}", response);
        let status = response.status();
        // Check if response status is OK
        if !status.is_success() {
            let content = response
                .text()
                .await
                .unwrap_or_else(|_| "No content".to_string());
            return Err(format!("HTTP Error: {}. {}", status, content).into());
        }

        // Get response text for debugging
        let response_text = response.text().await?;
        debug!("Response text: {}", response_text);

        // Try to parse the response
        match serde_json::from_str::<TelegramResponse<Vec<Update>>>(&response_text) {
            Ok(updates_response) => {
                if updates_response.ok {
                    Ok(updates_response.result)
                } else {
                    Err(updates_response
                        .description
                        .unwrap_or_else(|| "Unknown error".to_string())
                        .into())
                }
            }
            Err(parse_error) => {
                // Log the response text that failed to parse
                error!("Failed to parse response: {}", parse_error);
                error!("Response text: {}", response_text);
                Err(format!("Failed to parse Telegram API response: {}", parse_error).into())
            }
        }
    }
}
