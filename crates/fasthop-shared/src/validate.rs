use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};

pub const TOKEN_LEN: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("days must be between 1 and 3650")]
    Days,
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn is_well_formed_token(token: &str) -> bool {
    token.len() == TOKEN_LEN
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn validate_days(days: i32) -> Result<i32, ValidationError> {
    (1..=3650)
        .contains(&days)
        .then_some(days)
        .ok_or(ValidationError::Days)
}
