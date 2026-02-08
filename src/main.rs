use dotenv::dotenv;
use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, error};
use tracing_subscriber::{
    fmt::time::LocalTime,
    EnvFilter,
};
use time::macros::format_description;

mod telegram;
mod openobserve;
use telegram::*;
use openobserve::OpenObserve;

// --- Bot Configuration Functions ---

async fn delete_messages_after_delay(
    telegram_client: Arc<Telegram>,
    chat_id: i64,
    message_ids: Vec<u64>,
    delay_seconds: u64,
) {
    let telegram_client = Arc::clone(&telegram_client); // Clonamos el puntero, no el objeto entero
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(delay_seconds)).await;
        
        for message_id in message_ids {
            if let Err(e) = telegram_client.delete_message(chat_id, message_id).await {
                debug!("Failed to delete cleanup message {}: {}", message_id, e);
            } else {
                debug!("Cleanup: deleted message {} after {} seconds", message_id, delay_seconds);
            }
        }
    });
}

async fn get_or_create_bot_config(config_state: &BotConfigState, chat_id: i64) -> BotConfig {
    let mut state = config_state.lock().await;
    state.entry(chat_id).or_insert_with(|| BotConfig {
        whitelisted_bots: Vec::new(),
        admin_users: Vec::new(),
        notify_on_ban: true,
        banned_bots_count: 0,
    }).clone()
}

async fn is_user_admin(config_state: &BotConfigState, chat_id: i64, user_id: i64) -> bool {
    let state = config_state.lock().await;
    if let Some(config) = state.get(&chat_id) {
        config.admin_users.contains(&user_id)
    } else {
        false
    }
}

// --- Challenge Specific Functions ---

struct ChallengeDetails {
    correct_animal_name: String,
    challenge_message_id: u64,
    start_time: Instant,
    tx: oneshot::Sender<()>, // Channel to signal completion or timeout
}

// State: Map of chat_id -> (Map of user_id -> ChallengeDetails)
type ChallengeState = Arc<Mutex<HashMap<i64, HashMap<i64, ChallengeDetails>>>>;

// Bot whitelist and admin configuration
#[derive(Clone, Debug)]
struct BotConfig {
    whitelisted_bots: Vec<i64>,  // IDs de bots permitidos
    admin_users: Vec<i64>,       // IDs de administradores
    notify_on_ban: bool,         // Notificar cuando se expulsa un bot
    banned_bots_count: u64,      // Estadísticas de bots expulsados
}

type BotConfigState = Arc<Mutex<HashMap<i64, BotConfig>>>; // Por chat

// Define animal emojis and names for the challenge
const CHALLENGE_ANIMALS: &[(&str, &str)] = &[
    ("🐧", "penguin"),
    ("🐳", "whale"),
    ("🦀", "crab"),
    ("🦊", "fox"),
    ("🦭", "seal"),
    ("🐍", "snake"),
];

// --- Timer Task ---

async fn timer_task(
    telegram_client: Arc<Telegram>,
    chat_id: i64,
    user_id: i64,
    user_name: String,
    _challenge_message_id: u64,
    rx: oneshot::Receiver<()>, // Channel to receive signal for completion
    state: ChallengeState,
) {
    let challenge_duration_minutes = env::var("CHALLENGE_DURATION_MINUTES")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .unwrap_or(2);
    let challenge_duration = Duration::from_secs(challenge_duration_minutes * 60);

    let timer = sleep(challenge_duration);
    tokio::select! {
        _ = timer => {
            // Timer expired
            debug!("Challenge timer expired for user {} in chat {}", user_id, chat_id);
            let mut state_guard = state.lock().await;

            if let Some(user_challenges) = state_guard.get_mut(&chat_id) {
                if let Some(challenge) = user_challenges.get(&user_id) {

                    debug!("User {} did not respond in time. Banning.", user_id);
                    // Ban user
                    if let Err(e) = telegram_client.ban_chat_member(chat_id, user_id).await {
                        error!("Failed to ban user {}: {}", user_id, e);
                    } else {
                        let mut messages_to_delete = vec![challenge.challenge_message_id];
                        
                        // Send a notification and collect message ID
                        if let Ok(msg_id) = telegram_client.send_message(chat_id, &format!("El usuario {} fue expulsado por no completar el desafío.", user_name)).await {
                            messages_to_delete.push(msg_id);
                        }
                        
                        // Programar eliminación de mensajes después del tiempo configurado
                        let cleanup_delay = env::var("MESSAGE_CLEANUP_DELAY_SECONDS")
                            .unwrap_or_else(|_| "30".to_string())
                            .parse::<u64>()
                            .unwrap_or(30);
                            
                        delete_messages_after_delay(
                            telegram_client.clone(),
                            chat_id,
                            messages_to_delete,
                            cleanup_delay,
                        ).await;
                    }

                    // Remove from state
                    user_challenges.remove(&user_id);
                    // Clean up chat entry if no more users
                    if user_challenges.is_empty() {
                        state_guard.remove(&chat_id);
                    }
                }
            }
        },
        _ = rx => {
            // Challenge completed by user selecting a button
            debug!("Challenge completed by user {} in chat {}", user_id, chat_id);
            // The completion logic is handled in the callback query handler.
            // This branch is reached when the callback handler successfully signals completion.
        }
    }
}

// Function to process new members (both from chat_member updates and new_chat_members)
async fn process_new_member(
    telegram_client: Arc<Telegram>,
    chat_id: i64,
    user_id: i64,
    first_name: &str,
    challenge_state: &ChallengeState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!(
        "Processing new member: User ID {} in chat {}",
        user_id, chat_id
    );

    if telegram_client.restrict_chat_member(chat_id, user_id)
        .await
        .is_err()
    {
        error!(
            "Failed to restrict chat member {} in chat {}",
            user_id, chat_id
        );
        return Err("Failed to restrict member".into());
    }
    debug!("Permissions restricted for user {}", user_id);

    let mut rng = rand::rng();
    let (correct_emoji, correct_animal_name): (&str, &str) =
        *CHALLENGE_ANIMALS.choose(&mut rng).unwrap();

    let mut keyboard_buttons = Vec::new();
    for (emoji, animal_name) in CHALLENGE_ANIMALS {
        keyboard_buttons.push(InlineKeyboardButton {
            text: emoji.to_string(),
            url: None,
            callback_data: Some(animal_name.to_string()),
        });
    }

    let mut inline_keyboard = Vec::new();
    let mut row = Vec::new();
    for button in keyboard_buttons {
        row.push(button);
        if row.len() == 2 {
            inline_keyboard.push(row);
            row = Vec::new();
        }
    }
    if !row.is_empty() {
        inline_keyboard.push(row);
    }
    let markup = InlineKeyboardMarkup { inline_keyboard };

    let challenge_duration_minutes = env::var("CHALLENGE_DURATION_MINUTES")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .unwrap_or(2);

    let challenge_text = format!(
        "¡Bienvenido, <b>{}</b>!\nPara confirmar que eres un ser humano, selecciona este animal: {}\n\nTienes {} minutos.",
        first_name,
        correct_emoji,
        challenge_duration_minutes
    );

    match telegram_client.send_message_with_keyboard(chat_id, &challenge_text, markup).await {
        Ok(message_id) => {
            debug!(
                "Challenge message sent to user {} in chat {}: Message ID {}",
                user_id, chat_id, message_id
            );

            let (tx, rx) = oneshot::channel();
            let challenge_details = ChallengeDetails {
                correct_animal_name: correct_animal_name.to_string(),
                challenge_message_id: message_id,
                start_time: Instant::now(),
                tx,
            };

            let mut state_guard = challenge_state.lock().await;
            state_guard
                .entry(chat_id)
                .or_default()
                .insert(user_id, challenge_details);
            drop(state_guard);

            let state_clone = Arc::clone(&challenge_state);
            let telegram_client_clone = Arc::clone(&telegram_client);
            let first_name_clone = first_name.to_string();

            tokio::spawn(async move {
                timer_task(
                    telegram_client_clone,
                    chat_id,
                    user_id,
                    first_name_clone,
                    message_id,
                    rx,
                    state_clone,
                )
                .await;
            });

            Ok(())
        }
        Err(e) => {
            error!(
                "Failed to send challenge message for user {}: {}",
                user_id, e
            );
            if telegram_client.unrestrict_chat_member(chat_id, user_id)
                .await
                .is_err()
            {
                error!(
                    "Failed to unrestrict user {} after failed challenge message: {}",
                    user_id, e
                );
            }
            Err(e)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok(); // Load environment variables from .env file
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
    let open_observe_token = env::var("OPEN_OBSERVE_TOKEN").ok();

    let open_client = if let Some(url) = open_observe_url {
        if let Some(token) = open_observe_token {
            Some(OpenObserve::new(&url, "telegram_bot_challenges", &token))
        }else {
            None
        }
    } else {
        None
    };

    let mut offset = 0u64;

    let challenge_state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
    let bot_config_state: BotConfigState = Arc::new(Mutex::new(HashMap::new()));

    debug!("🚀 Bot started. Listening for updates...");

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
                                                        admin_users: Vec::new(),
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
                                                        if let Err(e) = telegram_client.send_message( message.chat.id, &notification_msg).await {
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
                                    &challenge_state,
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

                            if text.starts_with("/start") {
                                if telegram_client.send_message(
                                    message.chat.id,
                                    "¡Hola! Soy tu bot de Telegram. Úsame para administrar el acceso al grupo.",
                                )
                                .await
                                .is_err()
                                {
                                    error!(
                                        "Failed to send start message to chat {}",
                                        message.chat.id
                                    );
                                }
                            } else if text.starts_with("/help") {
                                let ban_bots_directly = env::var("BAN_BOTS_DIRECTLY")
                                    .unwrap_or_else(|_| "true".to_string())
                                    .to_lowercase() == "true";
                                
                                let bot_treatment = if ban_bots_directly {
                                    "Los bots son expulsados automáticamente"
                                } else {
                                    "Los bots reciben el mismo challenge que los usuarios"
                                };
                                
                                let help_text = format!(
                                    "🤖 **ExpulsaBot - Protección Anti-Bot**\n\n\
                                    📋 **Comandos disponibles:**\n\
                                    • /start - Iniciar el bot\n\
                                    • /help - Ver esta ayuda\n\
                                    • /whitelist <bot_id> - Permitir bot específico\n\
                                    • /unwhitelist <bot_id> - Remover bot de lista blanca\n\
                                    • /stats - Ver estadísticas del grupo\n\
                                    • /notify <on|off> - Activar/desactivar notificaciones\n\n\
                                    🔧 **Configuración actual:**\n\
                                    {}\n\n\
                                    👤 **Para usuarios humanos:**\n\
                                    Los nuevos miembros serán desafiados para verificar que no son bots.",
                                    bot_treatment
                                );
                                
                                if telegram_client.send_message( message.chat.id, &help_text).await.is_err() {
                                    error!("Failed to send help message to chat {}", message.chat.id);
                                }
                            } else if text.starts_with("/whitelist") {
                                let parts: Vec<&str> = text.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    if let Ok(bot_id) = parts[1].parse::<i64>() {
                                        let mut state = bot_config_state.lock().await;
                                        let config = state.entry(message.chat.id).or_insert_with(|| BotConfig {
                                            whitelisted_bots: Vec::new(),
                                            admin_users: Vec::new(),
                                            notify_on_ban: true,
                                            banned_bots_count: 0,
                                        });
                                        if !config.whitelisted_bots.contains(&bot_id) {
                                            config.whitelisted_bots.push(bot_id);
                                            if telegram_client.send_message( message.chat.id, &format!("✅ Bot {} agregado a la lista blanca", bot_id)).await.is_err() {
                                                error!("Failed to send whitelist confirmation");
                                            }
                                        } else if telegram_client.send_message( message.chat.id, &format!("⚠️ Bot {} ya está en la lista blanca", bot_id)).await.is_err() {
                                                error!("Failed to send whitelist warning");
                                        }
                                    }
                                } else if telegram_client.send_message(message.chat.id, "Uso: /whitelist <bot_id>").await.is_err() {
                                        error!("Failed to send whitelist usage");
                                }
                            } else if text.starts_with("/unwhitelist") {
                                let parts: Vec<&str> = text.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    if let Ok(bot_id) = parts[1].parse::<i64>() {
                                        let mut state = bot_config_state.lock().await;
                                        if let Some(config) = state.get_mut(&message.chat.id) {
                                            if let Some(pos) = config.whitelisted_bots.iter().position(|&x| x == bot_id) {
                                                config.whitelisted_bots.remove(pos);
                                                if telegram_client.send_message(message.chat.id, &format!("❌ Bot {} removido de la lista blanca", bot_id)).await.is_err() {
                                                    error!("Failed to send unwhitelist confirmation");
                                                }
                                            } else if telegram_client.send_message(message.chat.id, &format!("⚠️ Bot {} no está en la lista blanca", bot_id)).await.is_err() {
                                                    error!("Failed to send unwhitelist warning");
                                            }
                                        }
                                    }
                                } else if telegram_client.send_message(message.chat.id, "Uso: /unwhitelist <bot_id>").await.is_err() {
                                        error!("Failed to send unwhitelist usage");
                                }
                            } else if text.starts_with("/stats") {
                                let config = get_or_create_bot_config(&bot_config_state, message.chat.id).await;
                                let stats_msg = format!(
                                    "📊 **Estadísticas Anti-Bot**\n🤖 Bots expulsados: {}\n📝 Bots en lista blanca: {}\n🔔 Notificaciones: {}",
                                    config.banned_bots_count,
                                    config.whitelisted_bots.len(),
                                    if config.notify_on_ban { "Activadas" } else { "Desactivadas" }
                                );
                                if telegram_client.send_message(message.chat.id, &stats_msg).await.is_err() {
                                    error!("Failed to send stats message");
                                }
                            } else if text.starts_with("/notify") {
                                let parts: Vec<&str> = text.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    let enable = parts[1] == "on" || parts[1] == "true" || parts[1] == "1";
                                    let mut state = bot_config_state.lock().await;
                                    let config = state.entry(message.chat.id).or_insert_with(|| BotConfig {
                                        whitelisted_bots: Vec::new(),
                                        admin_users: Vec::new(),
                                        notify_on_ban: true,
                                        banned_bots_count: 0,
                                    });
                                    config.notify_on_ban = enable;
                                    let status = if enable { "activadas" } else { "desactivadas" };
                                    if telegram_client.send_message(message.chat.id, &format!("🔔 Notificaciones {}", status)).await.is_err() {
                                        error!("Failed to send notify confirmation");
                                    }
                                } else if telegram_client.send_message(message.chat.id, "Uso: /notify <on|off>").await.is_err() {
                                        error!("Failed to send notify usage");
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
                                &challenge_state,
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
                        if let (Some(message), Some(selected_animal)) =
                            (callback_query.message, callback_query.data)
                        {
                            let user_id = callback_query.from.id;
                            let chat_id = message.chat.id;
                            let mut state_guard = challenge_state.lock().await;
                            let mut challenge_removed = false;

                            if let Some(chat_challenges) = state_guard.get_mut(&chat_id) {
                                if let Some(challenge) = chat_challenges.remove(&user_id) {
                                    if selected_animal == challenge.correct_animal_name {
                                        debug!(
                                            "User {} selected the correct animal '{}' in chat {}",
                                            user_id, selected_animal, chat_id
                                        );

                                        let mut messages_to_delete = vec![challenge.challenge_message_id];

                                        if telegram_client.unrestrict_chat_member(chat_id, user_id) .await .is_err() {
                                            error!(
                                                "Failed to unrestrict chat member {} in chat {}",
                                                user_id, chat_id
                                            );
                                            if let Ok(msg_id) = telegram_client.send_message(chat_id, &format!("<b>{}</b> seleccionó el animal correcto, pero falló al otorgar permisos. Por favor contacta un administrador.", callback_query.from.first_name)).await {
                                                messages_to_delete.push(msg_id);
                                            }
                                        } else {
                                            debug!("Permissions granted for user {}", user_id);
                                            if let Ok(msg_id) = telegram_client.send_message(chat_id, &format!("<b>{}</b> ha pasado la verificación. ¡Bienvenido!", callback_query.from.first_name)).await {
                                                messages_to_delete.push(msg_id);
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
                                            "User {} selected the wrong animal '{}' in chat {}",
                                            user_id, selected_animal, chat_id
                                        );
                                        
                                        let mut messages_to_delete = vec![challenge.challenge_message_id];
                                        
                                        if let Ok(msg_id) = telegram_client.send_message(
                                            chat_id,
                                            "Ese no es el animal correcto. Has fallado el desafío.",
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
