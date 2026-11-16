use sha1::{Digest, Sha1};
use std::time::Instant;
use std::thread;

const MAX_PASSWORD_LENGTH: usize = 6;
const SECRET_PASSWORD: &str = "secret";

fn main() {
    let hash = generate_hash(SECRET_PASSWORD);
    let start = Instant::now();
    let result: Option<String> = crack(&hash);
    let duration = start.elapsed();
    match result {
        None => println!("Password not recovered."),
        Some(i) => println!("Your password is: {i}"),
    }
    println!("Execution time: {:?} s", duration);
}

fn byte_list_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_else(|_| String::new())
}

fn generate_hash(input: &str) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
}

fn crack_recursive(hash: &[u8], guess: &mut Vec<u8>) -> Option<String> {
    // Create a SHA1 digester
    let mut hasher = Sha1::new();
    hasher.update(&guess);
    let hash_guess = hasher.finalize();

    // If it's equal to the hash, return the guessed password
    if hash == hash_guess.as_slice() {
        return Some(byte_list_to_string(guess));
    }

    // If the current guess exceeds to max length, no need for further recursion
    if guess.len() >= MAX_PASSWORD_LENGTH {
        return None;
    }

    // Recursive case: add another letter
    for ch in b'a'..=b'z' {
        guess.push(ch);
        if let Some(result) = crack_recursive(hash, guess) {
            return Some(result);
        }
        guess.pop();
    }
    None
}

fn crack(hash: &[u8]) -> Option<String> {
    thread::scope(|scope| {
        let mut handles = Vec::new();

        for first in b'a'..=b'z' {
            handles.push(scope.spawn(move || {
                let mut guess = vec![first];
                crack_recursive(hash, &mut guess)
            }));
        }

        let mut result = None;

        for handle in handles {
            if let Some(password) = handle.join().unwrap() {
                result = Some(password);
            }
        }

        result
    })
}
