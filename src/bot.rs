use rand::prelude::*;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, error};
use uuid::Uuid;

use crate::matrix::Matrix;
use crate::openobserve::{OpenObserve, UserEvent};
use crate::telegram::*;

// --- Bot Configuration Functions ---

pub async fn delete_messages_after_delay(
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
                debug!(
                    "Cleanup: deleted message {} after {} seconds",
                    message_id, delay_seconds
                );
            }
        }
    });
}

pub async fn get_or_create_bot_config(config_state: &BotConfigState, chat_id: i64) -> BotConfig {
    let mut state = config_state.lock().await;
    state
        .entry(chat_id)
        .or_insert_with(|| BotConfig {
            whitelisted_bots: Vec::new(),
            notify_on_ban: true,
            banned_bots_count: 0,
        })
        .clone()
}

// --- Challenge Specific Functions ---

pub struct ChallengeDetails {
    pub correct_answer: String,
    pub challenge_message_id: u64,
    pub start_time: Instant,
    pub tx: oneshot::Sender<()>, // Channel to signal completion or timeout
}

// State: Map of chat_id -> (Map of user_id -> ChallengeDetails)
pub type ChallengeState = Arc<Mutex<HashMap<i64, HashMap<i64, ChallengeDetails>>>>;

// Bot whitelist and admin configuration
#[derive(Clone, Debug)]
pub struct BotConfig {
    pub whitelisted_bots: Vec<i64>, // IDs de bots permitidos
    pub notify_on_ban: bool,        // Notificar cuando se expulsa un bot
    pub banned_bots_count: u64,     // Estadísticas de bots expulsados
}

pub type BotConfigState = Arc<Mutex<HashMap<i64, BotConfig>>>; // Por chat

// Define number emojis for math challenge
const NUMBER_EMOJIS: &[&str] = &["0️⃣", "1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣"];

// Function to generate a math problem
pub fn generate_math_problem() -> (String, String, Vec<(String, String)>) {
    let mut rng = rand::rng();
    let a = rng.random_range(0..=9);
    let b = rng.random_range(0..=a);
    let correct_answer = a - b;

    // Create problem text with emojis
    let problem = format!("{} ➖ {} = ❓", NUMBER_EMOJIS[a], NUMBER_EMOJIS[b]);

    // Generate 5 unique numbers: 1 correct + 4 incorrect
    let mut all_numbers = std::collections::HashSet::new();
    all_numbers.insert(correct_answer); // Start with correct answer

    // Generate 4 different wrong answers
    while all_numbers.len() < 5 {
        let wrong = rng.random_range(0..=9);
        all_numbers.insert(wrong);
    }

    // Convert to vector and separate correct from incorrect
    let mut unique_numbers: Vec<usize> = all_numbers.into_iter().collect();

    // If somehow we don't have 5 unique numbers (shouldn't happen with 10 possible numbers)
    // but let's be safe and fill with remaining numbers
    while unique_numbers.len() < 5 {
        for num in 0..=9 {
            if !unique_numbers.contains(&num) {
                unique_numbers.push(num);
                if unique_numbers.len() >= 5 {
                    break;
                }
            }
        }
    }

    // Take only the first 5 numbers
    unique_numbers.truncate(5);

    // Separate correct answer from incorrect ones
    let correct_index = unique_numbers
        .iter()
        .position(|&x| x == correct_answer)
        .unwrap();
    unique_numbers.remove(correct_index);
    let incorrect_numbers = unique_numbers; // Now we have 4 incorrect numbers

    // Select first button (always incorrect)
    let first_wrong_index = rng.random_range(0..incorrect_numbers.len());
    let first_wrong = incorrect_numbers[first_wrong_index];

    // Prepare remaining numbers for positions 2-5 (3 incorrect + 1 correct)
    let mut remaining_numbers = Vec::new();
    for (i, &number) in incorrect_numbers.iter().enumerate() {
        if i != first_wrong_index {
            remaining_numbers.push(number);
        }
    }
    remaining_numbers.push(correct_answer); // Add correct answer

    // Shuffle positions 2-5
    remaining_numbers.shuffle(&mut rng);

    // Create buttons
    let mut answers = Vec::new();
    let mut correct_uuid = String::new();

    // First button (always wrong)
    let first_uuid = Uuid::new_v4().to_string();
    answers.push((NUMBER_EMOJIS[first_wrong].to_string(), first_uuid));

    // Remaining buttons (3 wrong + 1 correct, shuffled)
    for &number in &remaining_numbers {
        let uuid = Uuid::new_v4().to_string();
        if number == correct_answer {
            correct_uuid = uuid.clone();
        }
        answers.push((NUMBER_EMOJIS[number].to_string(), uuid));
    }

    (problem, correct_uuid, answers)
}

// --- Timer Task ---

pub async fn timer_task(
    telegram_client: Arc<Telegram>,
    chat_id: i64,
    user_id: i64,
    user_name: String,
    chat_title: Option<String>,
    _challenge_message_id: u64,
    rx: oneshot::Receiver<()>, // Channel to receive signal for completion
    state: ChallengeState,
    open_observe_client: Option<Arc<OpenObserve>>,
    matrix_client: Option<Arc<Matrix>>,
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

                        // Send event to OpenObserve
                        if let Some(open_client) = &open_observe_client {
                            let event = UserEvent {
                                user_id,
                                user_name: user_name.clone(),
                                group_id: chat_id,
                                group_name: chat_title.as_deref().unwrap_or("Unknown Group").to_string(),
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
                                user_name,
                                user_id,
                                chat_title.as_deref().unwrap_or("Unknown Group"),
                                chat_id
                            );
                            if let Err(e) = matrix_client.send_message(&matrix_message).await {
                                error!("Failed to send message to Matrix: {:?}", e);
                            }
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
pub async fn process_new_member(
    telegram_client: Arc<Telegram>,
    chat_id: i64,
    user_id: i64,
    first_name: &str,
    chat_title: Option<String>,
    challenge_state: &ChallengeState,
    open_observe_client: Option<Arc<OpenObserve>>,
    matrix_client: Option<Arc<Matrix>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!(
        "Processing new member: User ID {} in chat {}",
        user_id, chat_id
    );

    if telegram_client
        .restrict_chat_member(chat_id, user_id)
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

    let (problem_text, correct_uuid, answer_options) = generate_math_problem();

    let mut keyboard_buttons = Vec::new();
    for (emoji_text, uuid) in &answer_options {
        keyboard_buttons.push(InlineKeyboardButton {
            text: emoji_text.clone(),
            url: None,
            callback_data: Some(uuid.clone()),
        });
    }

    let mut inline_keyboard = Vec::new();
    let mut row = Vec::new();
    for button in keyboard_buttons {
        row.push(button);
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
        "¡Bienvenido, <b>{}</b>!\nPara confirmar que eres un ser humano, supera el desafío,\n{}\n\nTienes {} minutos.",
        first_name,
        problem_text,
        challenge_duration_minutes
    );

    match telegram_client
        .send_message_with_keyboard(chat_id, &challenge_text, markup)
        .await
    {
        Ok(message_id) => {
            debug!(
                "Challenge message sent to user {} in chat {}: Message ID {}",
                user_id, chat_id, message_id
            );

            let (tx, rx) = oneshot::channel();
            let challenge_details = ChallengeDetails {
                correct_answer: correct_uuid,
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
            let chat_title_clone = chat_title.clone();
            let open_observe_clone = open_observe_client.clone();
            let matrix_clone = matrix_client.clone();

            tokio::spawn(async move {
                timer_task(
                    telegram_client_clone,
                    chat_id,
                    user_id,
                    first_name_clone,
                    chat_title_clone,
                    message_id,
                    rx,
                    state_clone,
                    open_observe_clone,
                    matrix_clone,
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
            if telegram_client
                .unrestrict_chat_member(chat_id, user_id)
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
