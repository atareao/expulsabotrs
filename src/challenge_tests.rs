#![allow(unused_imports)] // Allow unused imports for now, as we are setting up tests

use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Instant, sleep, Duration};
use rand::seq::SliceRandom;

// Re-define or make ChallengeDetails and ChallengeState accessible for testing.
// For simplicity, we'll redefine them here. In a larger project, they would be in a shared module.

#[derive(Debug, Clone)] // Added Clone for easier testing
struct ChallengeDetails {
    correct_animal_name: String,
    challenge_message_id: u64,
    start_time: Instant,
    // For testing, we might not need the oneshot::Sender, or mock it.
    // For now, let's assume it's not directly tested here, or we use a dummy.
    // To make it testable, we can replace it with a dummy channel or ignore it.
    // For now, we'll just use a placeholder and won't send anything.
    // For a real scenario, a MockSender or a simple channel might be used.
    tx: tokio::sync::oneshot::Sender<()>, 
}

type ChallengeState = Arc<Mutex<HashMap<i64, HashMap<i64, ChallengeDetails>>>>;

// Helper to create a dummy oneshot sender for tests
fn dummy_oneshot_sender() -> tokio::sync::oneshot::Sender<()> {
    let (tx, _rx) = oneshot::channel();
    tx
}

// Mock Telegram API functions for testing, or assume they are called and focus on state management
// For now, we'll focus on the state management logic.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    // Mocking the Telegram API functions is complex. For now, we'll test the state management logic directly.
    // Tests for actual API interactions would require a mocking framework or integration tests.

    #[tokio::test]
    async fn test_challenge_state_add_and_get() {
        let state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
        let chat_id = 1001;
        let user_id = 2001;
        let message_id = 12345;
        let correct_animal = "penguin".to_string();

        let challenge_details = ChallengeDetails {
            correct_animal_name: correct_animal.clone(),
            challenge_message_id: message_id,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };

        // Add challenge
        let mut state_guard = state.lock().await;
        state_guard.entry(chat_id).or_default().insert(user_id, challenge_details.clone());
        drop(state_guard); // Release the lock

        // Get challenge
        let state_guard = state.lock().await;
        let user_challenges = state_guard.get(&chat_id);
        assert!(user_challenges.is_some(), "Chat challenges not found");
        let retrieved_challenge = user_challenges.unwrap().get(&user_id);
        assert!(retrieved_challenge.is_some(), "User challenge not found");

        let retrieved_challenge = retrieved_challenge.unwrap();
        assert_eq!(retrieved_challenge.correct_animal_name, correct_animal);
        assert_eq!(retrieved_challenge.challenge_message_id, message_id);
        // Check if start_time is reasonably close to now
        assert!(retrieved_challenge.start_time.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_challenge_state_remove() {
        let state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
        let chat_id = 1002;
        let user_id = 2002;
        let message_id = 54321;
        let correct_animal = "whale".to_string();

        let challenge_details = ChallengeDetails {
            correct_animal_name: correct_animal,
            challenge_message_id: message_id,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };

        // Add challenge
        let mut state_guard = state.lock().await;
        state_guard.entry(chat_id).or_default().insert(user_id, challenge_details);
        assert!(state_guard.contains_key(&chat_id));
        assert!(state_guard.get(&chat_id).unwrap().contains_key(&user_id));
        drop(state_guard);

        // Remove challenge (simulating successful completion or timeout)
        let mut state_guard = state.lock().await;
        if let Some(user_challenges) = state_guard.get_mut(&chat_id) {
            user_challenges.remove(&user_id);
            if user_challenges.is_empty() {
                state_guard.remove(&chat_id);
            }
        }
        drop(state_guard);

        // Verify removal
        let state_guard = state.lock().await;
        assert!(!state_guard.contains_key(&chat_id), "Chat entry should be removed if empty");
    }

    #[tokio::test]
    async fn test_challenge_state_multiple_users_and_chats() {
        let state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));

        let chat_id_1 = 1001;
        let user_id_1a = 2001;
        let user_id_1b = 2002;
        let chat_id_2 = 1002;
        let user_id_2a = 3001;

        // Add challenges for chat 1
        let challenge_1a = ChallengeDetails { correct_animal_name: "penguin".to_string(), challenge_message_id: 100, start_time: Instant::now(), tx: dummy_oneshot_sender() };
        let challenge_1b = ChallengeDetails { correct_animal_name: "whale".to_string(), challenge_message_id: 101, start_time: Instant::now(), tx: dummy_oneshot_sender() };
        // Add challenge for chat 2
        let challenge_2a = ChallengeDetails { correct_animal_name: "crab".to_string(), challenge_message_id: 200, start_time: Instant::now(), tx: dummy_oneshot_sender() };

        let mut state_guard = state.lock().await;
        state_guard.entry(chat_id_1).or_default().insert(user_id_1a, challenge_1a);
        state_guard.entry(chat_id_1).or_default().insert(user_id_1b, challenge_1b);
        state_guard.entry(chat_id_2).or_default().insert(user_id_2a, challenge_2a);
        drop(state_guard);

        // Verify counts
        let state_guard = state.lock().await;
        assert_eq!(state_guard.len(), 2, "Should have two chats");
        assert_eq!(state_guard.get(&chat_id_1).unwrap().len(), 2, "Chat 1 should have two users");
        assert_eq!(state_guard.get(&chat_id_2).unwrap().len(), 1, "Chat 2 should have one user");

        // Verify specific challenges
        assert_eq!(state_guard.get(&chat_id_1).unwrap().get(&user_id_1a).unwrap().correct_animal_name, "penguin");
        assert_eq!(state_guard.get(&chat_id_2).unwrap().get(&user_id_2a).unwrap().correct_animal_name, "crab");
        drop(state_guard);

        // Remove one challenge from chat 1
        let mut state_guard = state.lock().await;
        if let Some(chat_challenges) = state_guard.get_mut(&chat_id_1) {
            chat_challenges.remove(&user_id_1a);
            // Do not remove chat_id_1 entry yet, as user_id_1b is still there
        }
        drop(state_guard);

        // Verify removal
        let state_guard = state.lock().await;
        assert_eq!(state_guard.get(&chat_id_1).unwrap().len(), 1, "Chat 1 should now have one user");
        assert!(state_guard.get(&chat_id_1).unwrap().get(&user_id_1a).is_none(), "User 1a challenge should be removed");
        assert!(state_guard.get(&chat_id_1).unwrap().get(&user_id_1b).is_some(), "User 1b challenge should still be there");
        assert_eq!(state_guard.get(&chat_id_2).unwrap().len(), 1, "Chat 2 should still have one user");
    }

    #[tokio::test]
    async fn test_timer_task_expiration() {
        // This test requires mocking Instant::now() or using a specific duration that is guaranteed to pass.
        // For simplicity, we'll simulate a short duration and check if the state is cleaned up.
        
        // Mocking `Instant::now()` is tricky. A more robust approach would be to pass a mocked `Instant` or use a test utility.
        // For this test, we'll rely on the fact that `sleep` will eventually complete.
        // However, we can't easily test the *expiration* without actually waiting, which is slow for unit tests.
        // A better approach for testing expiration: reduce the duration to a very small amount (e.g., 10ms) and check if state is cleaned up.

        let state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
        let chat_id = 1003;
        let user_id = 2003;
        let message_id = 300;
        let correct_animal = "fox".to_string();

        let (tx, rx) = oneshot::channel();
        let challenge_details = ChallengeDetails {
            correct_animal_name: correct_animal,
            challenge_message_id: message_id,
            start_time: Instant::now(),
            tx,
        };

        // Add challenge to state
        let mut state_guard = state.lock().await;
        state_guard.entry(chat_id).or_default().insert(user_id, challenge_details);
        drop(state_guard);

        // Clone necessary items for the timer task
        let state_clone = Arc::clone(&state);
        let bot_token = "DUMMY_BOT_TOKEN".to_string(); // Mock token
        let client = Client::new(); // Real client, but API calls will fail/be ignored in test context

        // Spawn the timer task with a very short duration for testing
        // Note: This requires a way to control the duration dynamically or a test-specific setup.
        // For this example, we'll simulate the scenario where the timer expires.
        // A real test would likely need to mock `tokio::time::sleep`.
        
        // Since we cannot easily mock `sleep` or `Instant::now` to make the test fast and deterministic:
        // We will simulate the effect of the timer expiring by checking the state *after* a short sleep.
        // This test is not ideal for timing-critical logic.

        // To make this test practical, let's assume we modify timer_task to accept a custom duration for testing.
        // However, sticking to the current structure, we'll just sleep for a short period.
        // This test will be slow.

        // --- Simulating timer expiration --- 
        // In a real test, you'd likely mock `tokio::time::sleep` or `Instant::now` to make this fast.
        // For this example, we'll just sleep for a very short time to ensure `tokio::select` picks the timer branch.
        // This test is for demonstration and might be slow.

        let duration_for_test = Duration::from_millis(50); // Very short duration for test
        let mut simulated_timer = sleep(duration_for_test);
        
        // We need to also have a way to signal 'rx' completion if the user *had* responded.
        // Since we are testing expiration, we don't intend to signal 'rx'.
        // We can drop the receiver, or just not send anything on it.
        let _rx_to_drop = rx;

        // We need to simulate the API calls within timer_task failing or being a no-op in tests.
        // This requires mocking, which is beyond the scope of just writing the test file.

        // For demonstration, let's assume the timer task was spawned and executed.
        // We will manually check the state after a short delay.
        
        // For a practical test, we'd spawn the timer task and then check the state.
        // But `timer_task` takes `rx` which is consumed. So we can't easily run it and check.
        
        // A better test would be to mock the `ban_chat_member` and `delete_message` calls and ensure they are called.
        // And also to check that the challenge entry is removed from the state.

        println!("Simulating timer expiration for user {} in chat {}. This test is slow.", user_id, chat_id);
        sleep(duration_for_test).await;
        println!("Simulated delay passed.");

        // After the sleep, we expect the challenge to have been removed from state due to expiration.
        let state_guard = state.lock().await;
        assert!(state_guard.get(&chat_id).is_none(), "Chat entry should be removed after timer expiration");
    }

    // Note: Testing the callback query handling logic directly would involve:
    // 1. Creating a mock `CallbackQuery` object.
    // 2. Mocking the `unrestrict_chat_member`, `ban_chat_member`, `delete_message` API calls.
    // 3. Mocking the `tx.send(())` call.
    // 4. Calling a hypothetical `handle_callback_query` function with the mock data and state.
    // This is more involved and requires a proper mocking strategy.
}

// Mocking the actual Telegram API interaction functions for testing purposes.
// In a real scenario, you'd use a dedicated mocking library or framework.

// Mocking reqwest::Client and its methods is complex. For now, we'll focus on testing the state management.
// The main loop and timer task would need to be tested more thoroughly in an integration test environment.

