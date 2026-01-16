use argon2::{Argon2, PasswordHash, password_hash::{SaltString, rand_core::OsRng}};
use argon2::PasswordHasher;
use argon2::PasswordVerifier;

use crate::err::{AppResult, internal_error};

/// Hash plaintext using Argon2 and return the hash as a string.
pub fn hash(plaintext: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hashed = argon2
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|_| internal_error("Failed to hash password"))?;
    Ok(hashed.to_string())
}

/// Verify plaintext against a hash.
pub fn verify(password: &str, hash: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| internal_error("Could not read hash"))?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}