use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info};
use tracing_subscriber::{
    fmt::time::LocalTime,
    EnvFilter,
};
use time::macros::format_description;

mod telegram;
mod openobserve;
mod matrix;
pub mod bot;
mod commands;
//#[cfg(test)]
//mod challenge_tests;

use telegram::*;
use openobserve::{OpenObserve, UserEvent};
use matrix::Matrix;
use bot::{
    BotConfig, BotConfigState, ChallengeState,
    delete_messages_after_delay, get_or_create_bot_config, 
    process_new_member
};
use commands::handle_command;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok(); // Load environment variables from .env file
    
    // Record start time for status command
    let start_time = Instant::now();
    
    // El formato que querías
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    
    // LocalTime intentará leer la variable de entorno TZ o /etc/localtime
    let timer = LocalTime::new(format);

    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_env_filter(EnvFilter::from_default_env().add_directive("telegram_bot=debug".parse()?))
        .init();

    let token = env::var("TOKEN").expect("TOKEN not set in .env file");
    // Debug: show the token being used (mask sensitive part)
    let token_preview = if token.len() > 10 {
        format!("{}...", &token[..10])
    } else {
        "***".to_string()
    };
    debug!("Using bot token: {}", token_preview);
    let telegram_client = Arc::new(Telegram::new(&token));

    let open_observe_url = env::var("OPEN_OBSERVE_URL").ok();
    let open_observe_index = env::var("OPEN_OBSERVE_INDEX").ok();
    let open_observe_token = env::var("OPEN_OBSERVE_TOKEN").ok();

    let open_client = match (open_observe_url, open_observe_token, open_observe_index) {
        (Some(url), Some(token), Some(index)) => {
            debug!("OpenObserve integration enabled with URL: {}", url);
            Some(Arc::new(OpenObserve::new(&url, &index, &token)))
        }
        _ => None,
    };
    let matrix_url = env::var("MATRIX_URL").ok();
    let matrix_token = env::var("MATRIX_TOKEN").ok();
    let matrix_room= env::var("MATRIX_ROOM").ok();

    let matrix_client = match (matrix_url, matrix_token, matrix_room) {
        (Some(url), Some(token), Some(room)) => {
            debug!("Matrix integration enabled with URL: {}", url);
            Some(Arc::new(Matrix::new(&url, &token, &room)))
        }
        _ => None,
    };

    let mut offset = 0u64;

    let challenge_state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
    let bot_config_state: BotConfigState = Arc::new(Mutex::new(HashMap::new()));

    let version = env!("CARGO_PKG_VERSION");
    info!("🚀 Bot started");
    info!("📋 ExpulsaBot v{} initialized successfully", version);
    info!("👂️ Listening for updates...");

    loop {
        match telegram_client.get_updates( offset).await {
            Ok(updates) => {
                for update in updates {
                    offset = update.update_id + 1;

                    if let Some(message) = update.message {
                        // Check for new chat members in all possible fields
                        // Use HashSet to avoid processing the same user multiple times
                        let mut new_users_to_process: std::collections::HashSet<i64> = std::collections::HashSet::new();
                        let mut user_data_map: std::collections::HashMap<i64, &User> = std::collections::HashMap::new();

                        // Collect from new_chat_members (plural)
                        if let Some(new_members) = &message.new_chat_members {
                            for new_member in new_members {
                                debug!(
                                    "Detected new member via new_chat_members: {} (ID: {}, is_bot: {})", 
                                    new_member.first_name, new_member.id, new_member.is_bot
                                );
                                user_data_map.insert(new_member.id, new_member);
                                
                                if !new_member.is_bot {
                                    new_users_to_process.insert(new_member.id);
                                } else {
                                    // Verificar variable de entorno para el tratamiento de bots
                                    let ban_bots_directly = env::var("BAN_BOTS_DIRECTLY")
                                        .unwrap_or_else(|_| "true".to_string())
                                        .to_lowercase() == "true";
                                    
                                    if ban_bots_directly {
                                        // Verificar lista blanca
                                        let config = get_or_create_bot_config(&bot_config_state, message.chat.id).await;
                                        if config.whitelisted_bots.contains(&new_member.id) {
                                            debug!("Bot {} está en la lista blanca, permitiendo acceso", new_member.first_name);
                                        } else {
                                            debug!("Bot detected: {} - expulsando automáticamente", new_member.first_name);
                                            
                                            if let Err(e) = telegram_client.ban_chat_member(message.chat.id, new_member.id).await {
                                                error!("Failed to ban bot {}: {}", new_member.id, e);
                                            } else {
                                                debug!("Bot {} expulsado exitosamente", new_member.first_name);
                                                
                                                // Actualizar estadísticas y notificar
                                                {
                                                    let mut state = bot_config_state.lock().await;
                                                    let config = state.entry(message.chat.id).or_insert_with(|| BotConfig {
                                                        whitelisted_bots: Vec::new(),
                                                        notify_on_ban: true,
                                                        banned_bots_count: 0,
                                                    });
                                                    config.banned_bots_count += 1;
                                                    
                                                    if config.notify_on_ban {
                                                        let notification_msg = format!(
                                                            "🤖❌ Bot expulsado: {} (ID: {})\nTotal de bots expulsados: {}", 
                                                            new_member.first_name, 
                                                            new_member.id,
                                                            config.banned_bots_count
                                                        );
                                                        if let Err(e) = telegram_client.send_message(message.chat.id, &notification_msg).await {
                                                            error!("Failed to send ban notification: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // Tratar bot como usuario normal - aplicar challenge
                                        debug!("Bot detected: {} - aplicando challenge como usuario normal", new_member.first_name);
                                        new_users_to_process.insert(new_member.id);
                                    }
                                }
                            }
                        }

                        // Also check new_chat_member (singular) - only if not already processed
                        if let Some(new_member) = &message.new_chat_member {
                            if !user_data_map.contains_key(&new_member.id) {
                                debug!(
                                    "Detected new member via new_chat_member: {} (ID: {}, is_bot: {})",
                                    new_member.first_name, new_member.id, new_member.is_bot
                                );
                                user_data_map.insert(new_member.id, new_member);
                                
                                if !new_member.is_bot {
                                    new_users_to_process.insert(new_member.id);
                                } else {
                                    // Verificar variable de entorno para el tratamiento de bots
                                    let ban_bots_directly = env::var("BAN_BOTS_DIRECTLY")
                                        .unwrap_or_else(|_| "true".to_string())
                                        .to_lowercase() == "true";
                                    
                                    if ban_bots_directly {
                                        // Verificar lista blanca
                                        let config = get_or_create_bot_config(&bot_config_state, message.chat.id).await;
                                        if config.whitelisted_bots.contains(&new_member.id) {
                                            debug!("Bot {} está en la lista blanca, permitiendo acceso", new_member.first_name);
                                        } else {
                                            debug!("Bot detected: {} - expulsando automáticamente", new_member.first_name);
                                            if let Err(e) = telegram_client.ban_chat_member(message.chat.id, new_member.id).await {
                                                error!("Failed to ban bot {}: {}", new_member.id, e);
                                            } else {
                                                debug!("Bot {} expulsado exitosamente", new_member.first_name);
                                            }
                                        }
                                    } else {
                                        // Tratar bot como usuario normal - aplicar challenge
                                        debug!("Bot detected: {} - aplicando challenge como usuario normal", new_member.first_name);
                                        new_users_to_process.insert(new_member.id);
                                    }
                                }
                            } else {
                                debug!("User {} already processed from new_chat_members, skipping new_chat_member", new_member.id);
                            }
                        }

                        // Also check new_chat_participant (alternative field) - only if not already processed
                        if let Some(new_participant) = &message.new_chat_participant {
                            if !user_data_map.contains_key(&new_participant.id) {
                                debug!(
                                    "Detected new member via new_chat_participant: {} (ID: {}, is_bot: {})", 
                                    new_participant.first_name, new_participant.id, new_participant.is_bot
                                );
                                user_data_map.insert(new_participant.id, new_participant);
                                
                                if !new_participant.is_bot {
                                    new_users_to_process.insert(new_participant.id);
                                } else {
                                    // Verificar variable de entorno para el tratamiento de bots
                                    let ban_bots_directly = env::var("BAN_BOTS_DIRECTLY")
                                        .unwrap_or_else(|_| "true".to_string())
                                        .to_lowercase() == "true";
                                    
                                    if ban_bots_directly {
                                        // Verificar lista blanca
                                        let config = get_or_create_bot_config(&bot_config_state, message.chat.id).await;
                                        if config.whitelisted_bots.contains(&new_participant.id) {
                                            debug!("Bot {} está en la lista blanca, permitiendo acceso", new_participant.first_name);
                                        } else {
                                            debug!("Bot detected: {} - expulsando automáticamente", new_participant.first_name);
                                            if let Err(e) = telegram_client.ban_chat_member(message.chat.id, new_participant.id).await {
                                                error!("Failed to ban bot {}: {}", new_participant.id, e);
                                            } else {
                                                debug!("Bot {} expulsado exitosamente", new_participant.first_name);
                                            }
                                        }
                                    } else {
                                        // Tratar bot como usuario normal - aplicar challenge
                                        debug!("Bot detected: {} - aplicando challenge como usuario normal", new_participant.first_name);
                                        new_users_to_process.insert(new_participant.id);
                                    }
                                }
                            } else {
                                debug!("User {} already processed from previous fields, skipping new_chat_participant", new_participant.id);
                            }
                        }

                        // Process all collected new users (now without duplicates)
                        for user_id in new_users_to_process {
                            if let Some(user_data) = user_data_map.get(&user_id) {
                                debug!(
                                    "Processing unique new member: User ID {} in chat {}",
                                    user_id, message.chat.id
                                );
                                if let Err(e) = process_new_member(
                                    telegram_client.clone(),
                                    message.chat.id,
                                    user_id,
                                    &user_data.first_name,
                                    message.chat.title.clone(),
                                    &challenge_state,
                                    open_client.clone(),
                                    matrix_client.clone(),
                                )
                                .await
                                {
                                    error!("Failed to process new member {}: {}", user_id, e);
                                }
                            }
                        }

                        // Process text messages
                        if let Some(text) = message.text {
                            debug!(
                                "Received message in chat {}: '{}' from user {}",
                                message.chat.id, text, message.from.first_name
                            );

                            // Check if message is a command
                            if text.starts_with("/") {
                                let user_id = message.from.id;
                                let chat_id = message.chat.id;

                                // Handle command using the dedicated commands module
                                if let Err(e) = handle_command(
                                    &text,
                                    chat_id,
                                    user_id,
                                    &telegram_client,
                                    &bot_config_state,
                                    &start_time,
                                ).await {
                                    debug!("Command handling failed: {}", e);
                                }
                            }
                        }
                    } else if let Some(chat_member_update) = update.chat_member {
                        debug!("Chat Member Update received: {:?}", chat_member_update);

                        if chat_member_update.new_chat_member.status == "member"
                            && chat_member_update.old_chat_member.status != "member"
                        {
                            let user_id = chat_member_update.new_chat_member.user.id;
                            let chat_id = chat_member_update.chat.id;

                            if let Err(e) = process_new_member(
                                telegram_client.clone(),
                                chat_id,
                                user_id,
                                &chat_member_update.new_chat_member.user.first_name,
                                chat_member_update.chat.title.clone(),
                                &challenge_state,
                                open_client.clone(),
                                matrix_client.clone(),
                            )
                            .await
                            {
                                error!("Failed to process new member {}: {}", user_id, e);
                            }
                        }
                    } else if let Some(callback_query) = update.callback_query {
                        debug!(
                            "Callback query received: ID {}, From: {:?}",
                            callback_query.id,
                            callback_query
                                .from
                                .username
                                .as_ref()
                                .map_or(&callback_query.from.first_name, |u| u)
                        );
                        if let (Some(message), Some(selected_option)) =
                            (callback_query.message, callback_query.data)
                        {
                            let user_id = callback_query.from.id;
                            let chat_id = message.chat.id;
                            let mut state_guard = challenge_state.lock().await;
                            let mut challenge_removed = false;

                            if let Some(chat_challenges) = state_guard.get_mut(&chat_id) {
                                if let Some(challenge) = chat_challenges.remove(&user_id) {
                                    // Check if user responded too quickly (potential bot)
                                    let response_time = challenge.start_time.elapsed();
                                    let min_response_seconds = env::var("MIN_RESPONSE_SECONDS")
                                        .unwrap_or_else(|_| "1".to_string())
                                        .parse::<u64>()
                                        .unwrap_or(1);
                                    let min_response_time = Duration::from_secs(min_response_seconds);
                                    
                                    if response_time < min_response_time {
                                        debug!(
                                            "User {} responded too quickly ({:?} < {:?}) in chat {} - treating as bot",
                                            user_id, response_time, min_response_time, chat_id
                                        );
                                        
                                        let mut messages_to_delete = vec![challenge.challenge_message_id];
                                        
                                        if let Ok(msg_id) = telegram_client.send_message(
                                            chat_id,
                                            "Respuesta demasiado rápida. Comportamiento de bot detectado.",
                                        )
                                        .await {
                                            messages_to_delete.push(msg_id);
                                        }

                                        if telegram_client.ban_chat_member(chat_id, user_id)
                                            .await
                                            .is_err()
                                        {
                                            error!(
                                                "Failed to ban user {} for quick response in chat {}",
                                                user_id, chat_id
                                            );
                                        }

                                        // Send event to OpenObserve
                                        if let Some(open_observe_client) = &open_client {
                                            let event = UserEvent {
                                                user_id,
                                                user_name: callback_query.from.first_name.clone(),
                                                group_id: chat_id,
                                                group_name: message.chat.title.as_deref().unwrap_or("Unknown Group").to_string(),
                                                challenge_completed: false,
                                                banned: true,
                                            };
                                            if let Err(e) = open_observe_client.send_user_event(&event).await {
                                                error!("Failed to send user event to OpenObserve: {:?}", e);
                                            }
                                        }

                                        // Send message to Matrix
                                        if let Some(matrix_client) = &matrix_client {
                                            let matrix_message = format!(
                                                "el usuario {} con id {} respondió demasiado rápido ({:?}) y fue baneado del grupo {} con id {} por comportamiento de bot",
                                                callback_query.from.first_name,
                                                user_id,
                                                response_time,
                                                message.chat.title.as_deref().unwrap_or("Unknown Group"),
                                                chat_id
                                            );
                                            if let Err(e) = matrix_client.send_message(&matrix_message).await {
                                                error!("Failed to send message to Matrix: {:?}", e);
                                            }
                                        }
                                        
                                        // Schedule message deletion
                                        delete_messages_after_delay(
                                            telegram_client.clone(),
                                            chat_id,
                                            messages_to_delete,
                                            30,
                                        ).await;

                                        challenge_removed = true;
                                        let _ = challenge.tx.send(());
                                    } else if selected_option == challenge.correct_answer {
                                        debug!(
                                            "User {} selected the correct answer '{}' in chat {}",
                                            user_id, selected_option, chat_id
                                        );

                                        let mut messages_to_delete = vec![challenge.challenge_message_id];

                                        if telegram_client.unrestrict_chat_member(chat_id, user_id) .await .is_err() {
                                            error!(
                                                "Failed to unrestrict chat member {} in chat {}",
                                                user_id, chat_id
                                            );
                                            if let Ok(msg_id) = telegram_client.send_message(chat_id, &format!("<b>{}</b> seleccionó la respuesta correcta, pero falló al otorgar permisos. Por favor contacta un administrador.", callback_query.from.first_name)).await {
                                                messages_to_delete.push(msg_id);
                                            }
                                        } else {
                                            debug!("Permissions granted for user {}", user_id);
                                            if let Ok(msg_id) = telegram_client.send_message(chat_id, &format!("<b>{}</b> ha pasado la verificación. ¡Bienvenido!", callback_query.from.first_name)).await {
                                                messages_to_delete.push(msg_id);
                                            }
                                        }

                                        // Send success event to OpenObserve
                                        if let Some(open_client) = &open_client {
                                            let event = UserEvent {
                                                user_id,
                                                user_name: callback_query.from.first_name.clone(),
                                                group_id: chat_id,
                                                group_name: message.chat.title.as_deref().unwrap_or("Unknown Group").to_string(),
                                                challenge_completed: true,
                                                banned: false,
                                            };
                                            if let Err(e) = open_client.send_user_event(&event).await {
                                                error!("Failed to send user event to OpenObserve: {:?}", e);
                                            }
                                        }

                                        // Send message to Matrix
                                        if let Some(matrix_client) = &matrix_client {
                                            let matrix_message = format!(
                                                "el usuario {} con id {} si superó el challenge y no fue baneado del grupo {} con id {}",
                                                callback_query.from.first_name,
                                                user_id,
                                                message.chat.title.as_deref().unwrap_or("Unknown Group"),
                                                chat_id
                                            );
                                            if let Err(e) = matrix_client.send_message(&matrix_message).await {
                                                error!("Failed to send message to Matrix: {:?}", e);
                                            }
                                        }

                                        // Programar eliminación de mensajes después de 30 segundos
                                        delete_messages_after_delay(
                                            telegram_client.clone(),
                                            chat_id,
                                            messages_to_delete,
                                            30,
                                        ).await;

                                        challenge_removed = true;
                                        let _ = challenge.tx.send(());
                                    } else {
                                        debug!(
                                            "User {} selected the wrong answer '{}' in chat {}",
                                            user_id, selected_option, chat_id
                                        );
                                        
                                        let mut messages_to_delete = vec![challenge.challenge_message_id];
                                        
                                        if let Ok(msg_id) = telegram_client.send_message(
                                            chat_id,
                                            "Esa no es la respuesta correcta. Has fallado el desafío.",
                                        )
                                        .await {
                                            messages_to_delete.push(msg_id);
                                        }

                                        if telegram_client.ban_chat_member(chat_id, user_id)
                                            .await
                                            .is_err()
                                        {
                                            error!(
                                                "Failed to ban user {} after incorrect answer: {}",
                                                user_id, "Error"
                                            );
                                        }

                                        // Send failure event to OpenObserve
                                        if let Some(open_client) = &open_client {
                                            let event = UserEvent {
                                                user_id,
                                                user_name: callback_query.from.first_name.clone(),
                                                group_id: chat_id,
                                                group_name: message.chat.title.as_deref().unwrap_or("Unknown Group").to_string(),
                                                challenge_completed: false,
                                                banned: true,
                                            };
                                            if let Err(e) = open_client.send_user_event(&event).await {
                                                error!("Failed to send user event to OpenObserve: {:?}", e);
                                            }
                                        }

                                        // Send message to Matrix
                                        if let Some(matrix_client) = &matrix_client {
                                            let matrix_message = format!(
                                                "el usuario {} con id {} no superó el challenge y fue baneado del grupo {} con id {}",
                                                callback_query.from.first_name,
                                                user_id,
                                                message.chat.title.as_deref().unwrap_or("Unknown Group"),
                                                chat_id
                                            );
                                            if let Err(e) = matrix_client.send_message(&matrix_message).await {
                                                error!("Failed to send message to Matrix: {:?}", e);
                                            }
                                        }
                                        
                                        // Programar eliminación de mensajes después de 30 segundos
                                        delete_messages_after_delay(
                                            telegram_client.clone(),
                                            chat_id,
                                            messages_to_delete,
                                            30,
                                        ).await;

                                        challenge_removed = true;
                                        let _ = challenge.tx.send(());
                                    }
                                }
                            }

                            if challenge_removed {
                                if let Some(chat_challenges) = state_guard.get_mut(&chat_id) {
                                    if chat_challenges.is_empty() {
                                        state_guard.remove(&chat_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error fetching updates: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}
