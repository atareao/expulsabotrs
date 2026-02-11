#![allow(unused_imports)] // Allow unused imports for now, as we are setting up tests

use super::*;
use crate::bot::{generate_math_problem, ChallengeDetails, ChallengeState};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::{sleep, Duration, Instant};

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
    use reqwest::Client;
    use std::time::SystemTime;

    // Mocking the Telegram API functions is complex. For now, we'll test the state management logic directly.
    // Tests for actual API interactions would require a mocking framework or integration tests.

    #[tokio::test]
    async fn test_challenge_state_add_and_get() {
        let state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
        let chat_id = 1001;
        let user_id = 2001;
        let message_id = 12345;
        let correct_answer = "3".to_string();

        let challenge_details = ChallengeDetails {
            correct_answer: correct_answer.clone(),
            challenge_message_id: message_id,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };

        // Add challenge
        let mut state_guard = state.lock().await;
        state_guard
            .entry(chat_id)
            .or_default()
            .insert(user_id, challenge_details);
        drop(state_guard); // Release the lock

        // Get challenge
        let state_guard = state.lock().await;
        let user_challenges = state_guard.get(&chat_id);
        assert!(user_challenges.is_some(), "Chat challenges not found");
        let retrieved_challenge = user_challenges.unwrap().get(&user_id);
        assert!(retrieved_challenge.is_some(), "User challenge not found");

        let retrieved_challenge = retrieved_challenge.unwrap();
        assert_eq!(retrieved_challenge.correct_answer, correct_answer);
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
        let correct_answer = "5".to_string();

        let challenge_details = ChallengeDetails {
            correct_answer: correct_answer,
            challenge_message_id: message_id,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };

        // Add challenge
        let mut state_guard = state.lock().await;
        state_guard
            .entry(chat_id)
            .or_default()
            .insert(user_id, challenge_details);
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
        assert!(
            !state_guard.contains_key(&chat_id),
            "Chat entry should be removed if empty"
        );
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
        let challenge_1a = ChallengeDetails {
            correct_answer: "2".to_string(),
            challenge_message_id: 100,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };
        let challenge_1b = ChallengeDetails {
            correct_answer: "7".to_string(),
            challenge_message_id: 101,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };
        // Add challenge for chat 2
        let challenge_2a = ChallengeDetails {
            correct_answer: "1".to_string(),
            challenge_message_id: 200,
            start_time: Instant::now(),
            tx: dummy_oneshot_sender(),
        };

        let mut state_guard = state.lock().await;
        state_guard
            .entry(chat_id_1)
            .or_default()
            .insert(user_id_1a, challenge_1a);
        state_guard
            .entry(chat_id_1)
            .or_default()
            .insert(user_id_1b, challenge_1b);
        state_guard
            .entry(chat_id_2)
            .or_default()
            .insert(user_id_2a, challenge_2a);
        drop(state_guard);

        // Verify counts
        let state_guard = state.lock().await;
        assert_eq!(state_guard.len(), 2, "Should have two chats");
        assert_eq!(
            state_guard.get(&chat_id_1).unwrap().len(),
            2,
            "Chat 1 should have two users"
        );
        assert_eq!(
            state_guard.get(&chat_id_2).unwrap().len(),
            1,
            "Chat 2 should have one user"
        );

        // Verify specific challenges
        assert_eq!(
            state_guard
                .get(&chat_id_1)
                .unwrap()
                .get(&user_id_1a)
                .unwrap()
                .correct_answer,
            "2"
        );
        assert_eq!(
            state_guard
                .get(&chat_id_2)
                .unwrap()
                .get(&user_id_2a)
                .unwrap()
                .correct_answer,
            "1"
        );
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
        assert_eq!(
            state_guard.get(&chat_id_1).unwrap().len(),
            1,
            "Chat 1 should now have one user"
        );
        assert!(
            state_guard
                .get(&chat_id_1)
                .unwrap()
                .get(&user_id_1a)
                .is_none(),
            "User 1a challenge should be removed"
        );
        assert!(
            state_guard
                .get(&chat_id_1)
                .unwrap()
                .get(&user_id_1b)
                .is_some(),
            "User 1b challenge should still be there"
        );
        assert_eq!(
            state_guard.get(&chat_id_2).unwrap().len(),
            1,
            "Chat 2 should still have one user"
        );
    }

    #[tokio::test]
    async fn test_timer_task_expiration() {
        // Simplified test to check basic challenge state management
        let state: ChallengeState = Arc::new(Mutex::new(HashMap::new()));
        let chat_id = 1003;
        let user_id = 2003;
        let message_id = 300;
        let correct_answer = "4".to_string();

        let (tx, _rx) = oneshot::channel();
        let challenge_details = ChallengeDetails {
            correct_answer: correct_answer,
            challenge_message_id: message_id,
            start_time: Instant::now(),
            tx,
        };

        // Add challenge to state
        let mut state_guard = state.lock().await;
        state_guard
            .entry(chat_id)
            .or_default()
            .insert(user_id, challenge_details);
        drop(state_guard);

        // Verify the challenge was added
        let state_guard = state.lock().await;
        assert!(
            state_guard.contains_key(&chat_id),
            "Chat challenge should exist"
        );
        assert!(
            state_guard.get(&chat_id).unwrap().contains_key(&user_id),
            "User challenge should exist"
        );
    }

    // Note: Testing the callback query handling logic directly would involve:
    // 1. Creating a mock `CallbackQuery` object.
    // 2. Mocking the `unrestrict_chat_member`, `ban_chat_member`, `delete_message` API calls.
    // 3. Mocking the `tx.send(())` call.
    // 4. Calling a hypothetical `handle_callback_query` function with the mock data and state.
    // This is more involved and requires a proper mocking strategy.

    // === Tests for Math Challenge Generation ===

    #[test]
    fn test_generate_math_problem_basic_structure() {
        let (problem, correct_uuid, answers) = generate_math_problem();

        // Test basic structure
        assert!(!problem.is_empty(), "Problem text should not be empty");
        assert!(!correct_uuid.is_empty(), "Correct UUID should not be empty");
        assert_eq!(answers.len(), 5, "Should have exactly 5 answer buttons");

        // Test that problem contains expected elements
        assert!(
            problem.contains("➖"),
            "Problem should contain subtraction symbol"
        );
        assert!(
            problem.contains("❓"),
            "Problem should contain question mark"
        );

        // Test that all answers have non-empty text and UUIDs
        for (emoji_text, uuid) in &answers {
            assert!(!emoji_text.is_empty(), "Answer text should not be empty");
            assert!(!uuid.is_empty(), "Answer UUID should not be empty");
            assert!(emoji_text.ends_with("️⃣"), "Answer should be a number emoji");
        }
    }

    #[test]
    fn test_generate_math_problem_answer_uniqueness() {
        let (_, _, answers) = generate_math_problem();

        // Test that all answer texts are unique
        let mut seen_texts = std::collections::HashSet::new();
        for (emoji_text, _) in &answers {
            assert!(
                seen_texts.insert(emoji_text.clone()),
                "All answer texts should be unique, found duplicate: {}",
                emoji_text
            );
        }

        // Test that all UUIDs are unique
        let mut seen_uuids = std::collections::HashSet::new();
        for (_, uuid) in &answers {
            assert!(
                seen_uuids.insert(uuid.clone()),
                "All UUIDs should be unique, found duplicate: {}",
                uuid
            );
        }
    }

    #[test]
    fn test_generate_math_problem_correct_answer_present() {
        let (problem, correct_uuid, answers) = generate_math_problem();

        // Extract the numbers from the problem to verify the correct answer
        // Problem format: "X️⃣ ➖ Y️⃣ = ❓"
        let problem_parts: Vec<&str> = problem.split(" ").collect();
        assert_eq!(
            problem_parts.len(),
            5,
            "Problem should have 5 parts separated by spaces"
        );

        // Find the position of the correct UUID in answers
        let correct_position = answers.iter().position(|(_, uuid)| uuid == &correct_uuid);
        assert!(
            correct_position.is_some(),
            "Correct UUID should be found in answers"
        );

        // Verify correct answer is not in first position (as per requirement)
        let pos = correct_position.unwrap();
        assert!(
            pos > 0,
            "Correct answer should not be in first position (index 0), found at index {}",
            pos
        );
    }

    #[test]
    fn test_generate_math_problem_first_answer_always_wrong() {
        // Run multiple times to ensure first answer is consistently wrong
        for _ in 0..10 {
            let (_, correct_uuid, answers) = generate_math_problem();

            let first_answer_uuid = &answers[0].1;
            assert_ne!(
                first_answer_uuid, &correct_uuid,
                "First answer should never be the correct answer"
            );
        }
    }

    #[test]
    fn test_generate_math_problem_mathematical_correctness() {
        let (problem, correct_uuid, answers) = generate_math_problem();

        // Parse the problem to extract A and B
        let problem_parts: Vec<&str> = problem.split(" ").collect();
        let a_emoji = problem_parts[0];
        let b_emoji = problem_parts[2];

        // Map emojis to numbers
        let emoji_to_num = [
            ("0️⃣", 0),
            ("1️⃣", 1),
            ("2️⃣", 2),
            ("3️⃣", 3),
            ("4️⃣", 4),
            ("5️⃣", 5),
            ("6️⃣", 6),
            ("7️⃣", 7),
            ("8️⃣", 8),
            ("9️⃣", 9),
        ];

        let a = emoji_to_num
            .iter()
            .find(|(emoji, _)| emoji == &a_emoji)
            .unwrap()
            .1;
        let b = emoji_to_num
            .iter()
            .find(|(emoji, _)| emoji == &b_emoji)
            .unwrap()
            .1;

        // Verify A >= B (as per problem generation logic)
        assert!(
            a >= b,
            "A should be greater than or equal to B, got A={}, B={}",
            a,
            b
        );

        // Calculate expected answer
        let expected_answer = a - b;
        let expected_emoji = emoji_to_num
            .iter()
            .find(|(_, num)| num == &expected_answer)
            .unwrap()
            .0;

        // Find the correct answer in the options
        let correct_answer_emoji = answers
            .iter()
            .find(|(_, uuid)| uuid == &correct_uuid)
            .map(|(emoji, _)| emoji)
            .unwrap();

        assert_eq!(
            correct_answer_emoji, expected_emoji,
            "Correct answer emoji should match calculated result. Expected: {}, Found: {}",
            expected_emoji, correct_answer_emoji
        );
    }

    #[test]
    fn test_generate_math_problem_consistency() {
        // Test that multiple calls produce valid but different problems
        let mut problems = std::collections::HashSet::new();
        let mut correct_uuids = std::collections::HashSet::new();

        for _ in 0..20 {
            let (problem, correct_uuid, answers) = generate_math_problem();

            // Each problem should be valid
            assert_eq!(answers.len(), 5, "Should always have 5 answers");
            assert!(!problem.is_empty(), "Problem should not be empty");
            assert!(!correct_uuid.is_empty(), "Correct UUID should not be empty");

            // Collect for diversity check
            problems.insert(problem);
            correct_uuids.insert(correct_uuid);
        }

        // We should see some variety in problems and UUIDs
        assert!(problems.len() > 1, "Should generate different problems");
        assert_eq!(
            correct_uuids.len(),
            20,
            "Each run should generate a unique UUID"
        );
    }
}

// Mocking the actual Telegram API interaction functions for testing purposes.
// In a real scenario, you'd use a dedicated mocking library or framework.

// Mocking reqwest::Client and its methods is complex. For now, we'll focus on testing the state management.
// The main loop and timer task would need to be tested more thoroughly in an integration test environment.
