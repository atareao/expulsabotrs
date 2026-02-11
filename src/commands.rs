use std::env;
use std::sync::Arc;
use tokio::time::Instant;
use tracing::error;

use crate::bot::{get_or_create_bot_config, BotConfig, BotConfigState};
use crate::telegram::Telegram;

pub async fn handle_command(
    text: &str,
    chat_id: i64,
    user_id: i64,
    telegram_client: &Arc<Telegram>,
    bot_config_state: &BotConfigState,
    start_time: &Instant,
) -> Result<(), String> {
    // For all commands, check if user is admin using Telegram API
    match telegram_client.is_chat_admin(chat_id, user_id).await {
        Ok(is_admin) => {
            if !is_admin {
                if telegram_client
                    .send_message(
                        chat_id,
                        "❌ Solo los administradores del grupo pueden usar comandos del bot",
                    )
                    .await
                    .is_err()
                {
                    error!("Failed to send permission denied message");
                }
                return Err("User is not admin".to_string());
            }
        }
        Err(e) => {
            error!("Failed to check admin status for user {}: {}", user_id, e);
            if telegram_client
                .send_message(chat_id, "❌ Error al verificar permisos de administrador")
                .await
                .is_err()
            {
                error!("Failed to send admin check error message");
            }
            return Err(format!("Admin check failed: {}", e));
        }
    }

    // Process admin-only commands
    if text.starts_with("/start") {
        handle_start_command(chat_id, telegram_client).await
    } else if text.starts_with("/help") {
        handle_help_command(chat_id, telegram_client).await
    } else if text.starts_with("/status") {
        handle_status_command(chat_id, telegram_client, start_time).await
    } else if text.starts_with("/whitelist") {
        handle_whitelist_command(text, chat_id, telegram_client, bot_config_state).await
    } else if text.starts_with("/unwhitelist") {
        handle_unwhitelist_command(text, chat_id, telegram_client, bot_config_state).await
    } else if text.starts_with("/stats") {
        handle_stats_command(chat_id, telegram_client, bot_config_state).await
    } else if text.starts_with("/notify") {
        handle_notify_command(text, chat_id, telegram_client, bot_config_state).await
    } else {
        Ok(()) // Unknown command, do nothing
    }
}

async fn handle_start_command(chat_id: i64, telegram_client: &Arc<Telegram>) -> Result<(), String> {
    if telegram_client
        .send_message(
            chat_id,
            "¡Hola! Soy tu bot de Telegram. Úsame para administrar el acceso al grupo.",
        )
        .await
        .is_err()
    {
        error!("Failed to send start message to chat {}", chat_id);
        return Err("Failed to send start message".to_string());
    }
    Ok(())
}

async fn handle_help_command(chat_id: i64, telegram_client: &Arc<Telegram>) -> Result<(), String> {
    let ban_bots_directly = env::var("BAN_BOTS_DIRECTLY")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase()
        == "true";

    let bot_treatment = if ban_bots_directly {
        "Los bots son expulsados automáticamente"
    } else {
        "Los bots reciben el mismo challenge que los usuarios"
    };

    let help_text = format!(
        "🤖 <b>ExpulsaBot - Protección Anti-Bot</b>\n\n\
        📋 <b>Comandos disponibles:</b>\n\
        • /start - Iniciar el bot\n\
        • /help - Ver esta ayuda\n\
        • /status - Ver estado del bot\n\
        • /whitelist &lt;bot_id&gt; - Permitir bot específico\n\
        • /unwhitelist &lt;bot_id&gt; - Remover bot de lista blanca\n\
        • /stats - Ver estadísticas del grupo\n\
        • /notify &lt;on|off&gt; - Activar/desactivar notificaciones\n\n\
        🔧 <b>Configuración actual:</b>\n\
        {}\n\n\
        ⚠️ <b>Importante:</b>\n\
        Solo los administradores de Telegram pueden usar comandos del bot.\n\n\
        👤 <b>Para usuarios humanos:</b>\n\
        Los nuevos miembros serán desafiados para verificar que no son bots.",
        bot_treatment
    );

    if telegram_client
        .send_message(chat_id, &help_text)
        .await
        .is_err()
    {
        error!("Failed to send help message to chat {}", chat_id);
        return Err("Failed to send help message".to_string());
    }
    Ok(())
}

async fn handle_status_command(
    chat_id: i64,
    telegram_client: &Arc<Telegram>,
    start_time: &Instant,
) -> Result<(), String> {
    let uptime = start_time.elapsed();
    let total_seconds = uptime.as_secs();

    let status_text = if total_seconds < 60 {
        format!(
            "🟢 <b>Bot Estado:</b> Activo\n⏱️ <b>Tiempo en línea:</b> {} segundos",
            total_seconds
        )
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!(
            "🟢 <b>Bot Estado:</b> Activo\n⏱️ <b>Tiempo en línea:</b> {} minutos y {} segundos",
            minutes, seconds
        )
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        format!(
            "🟢 <b>Bot Estado:</b> Activo\n⏱️ <b>Tiempo en línea:</b> {} horas y {} minutos",
            hours, minutes
        )
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        format!(
            "🟢 <b>Bot Estado:</b> Activo\n⏱️ <b>Tiempo en línea:</b> {} días y {} horas",
            days, hours
        )
    };

    if telegram_client
        .send_message(chat_id, &status_text)
        .await
        .is_err()
    {
        error!("Failed to send status message to chat {}", chat_id);
        return Err("Failed to send status message".to_string());
    }
    Ok(())
}

async fn handle_whitelist_command(
    text: &str,
    chat_id: i64,
    telegram_client: &Arc<Telegram>,
    bot_config_state: &BotConfigState,
) -> Result<(), String> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(bot_id) = parts[1].parse::<i64>() {
            let mut state = bot_config_state.lock().await;
            let config = state.entry(chat_id).or_insert_with(|| BotConfig {
                whitelisted_bots: Vec::new(),
                notify_on_ban: true,
                banned_bots_count: 0,
            });
            if !config.whitelisted_bots.contains(&bot_id) {
                config.whitelisted_bots.push(bot_id);
                if telegram_client
                    .send_message(
                        chat_id,
                        &format!("✅ Bot {} agregado a la lista blanca", bot_id),
                    )
                    .await
                    .is_err()
                {
                    error!("Failed to send whitelist confirmation");
                    return Err("Failed to send whitelist confirmation".to_string());
                }
            } else if telegram_client
                .send_message(
                    chat_id,
                    &format!("⚠️ Bot {} ya está en la lista blanca", bot_id),
                )
                .await
                .is_err()
            {
                error!("Failed to send whitelist warning");
                return Err("Failed to send whitelist warning".to_string());
            }
        }
    } else if telegram_client
        .send_message(chat_id, "Uso: /whitelist <bot_id>")
        .await
        .is_err()
    {
        error!("Failed to send whitelist usage");
        return Err("Failed to send whitelist usage".to_string());
    }
    Ok(())
}

async fn handle_unwhitelist_command(
    text: &str,
    chat_id: i64,
    telegram_client: &Arc<Telegram>,
    bot_config_state: &BotConfigState,
) -> Result<(), String> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(bot_id) = parts[1].parse::<i64>() {
            let mut state = bot_config_state.lock().await;
            if let Some(config) = state.get_mut(&chat_id) {
                if let Some(pos) = config.whitelisted_bots.iter().position(|&x| x == bot_id) {
                    config.whitelisted_bots.remove(pos);
                    if telegram_client
                        .send_message(
                            chat_id,
                            &format!("❌ Bot {} removido de la lista blanca", bot_id),
                        )
                        .await
                        .is_err()
                    {
                        error!("Failed to send unwhitelist confirmation");
                        return Err("Failed to send unwhitelist confirmation".to_string());
                    }
                } else if telegram_client
                    .send_message(
                        chat_id,
                        &format!("⚠️ Bot {} no está en la lista blanca", bot_id),
                    )
                    .await
                    .is_err()
                {
                    error!("Failed to send unwhitelist warning");
                    return Err("Failed to send unwhitelist warning".to_string());
                }
            }
        }
    } else if telegram_client
        .send_message(chat_id, "Uso: /unwhitelist <bot_id>")
        .await
        .is_err()
    {
        error!("Failed to send unwhitelist usage");
        return Err("Failed to send unwhitelist usage".to_string());
    }
    Ok(())
}

async fn handle_stats_command(
    chat_id: i64,
    telegram_client: &Arc<Telegram>,
    bot_config_state: &BotConfigState,
) -> Result<(), String> {
    let config = get_or_create_bot_config(bot_config_state, chat_id).await;
    let stats_msg = format!(
        "📊 <b>Estadísticas Anti-Bot</b>\n🤖 Bots expulsados: {}\n📝 Bots en lista blanca: {}\n🔔 Notificaciones: {}",
        config.banned_bots_count,
        config.whitelisted_bots.len(),
        if config.notify_on_ban { "Activadas" } else { "Desactivadas" }
    );
    if telegram_client
        .send_message(chat_id, &stats_msg)
        .await
        .is_err()
    {
        error!("Failed to send stats message");
        return Err("Failed to send stats message".to_string());
    }
    Ok(())
}

async fn handle_notify_command(
    text: &str,
    chat_id: i64,
    telegram_client: &Arc<Telegram>,
    bot_config_state: &BotConfigState,
) -> Result<(), String> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() >= 2 {
        let enable = parts[1] == "on" || parts[1] == "true" || parts[1] == "1";
        let mut state = bot_config_state.lock().await;
        let config = state.entry(chat_id).or_insert_with(|| BotConfig {
            whitelisted_bots: Vec::new(),
            notify_on_ban: true,
            banned_bots_count: 0,
        });
        config.notify_on_ban = enable;
        let status = if enable { "activadas" } else { "desactivadas" };
        if telegram_client
            .send_message(chat_id, &format!("🔔 Notificaciones {}", status))
            .await
            .is_err()
        {
            error!("Failed to send notify confirmation");
            return Err("Failed to send notify confirmation".to_string());
        }
    } else if telegram_client
        .send_message(chat_id, "Uso: /notify <on|off>")
        .await
        .is_err()
    {
        error!("Failed to send notify usage");
        return Err("Failed to send notify usage".to_string());
    }
    Ok(())
}
