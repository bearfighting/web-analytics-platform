use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use thiserror::Error;

const KEY_BYTES: usize = 32;

#[derive(Debug, Error)]
pub enum KeyGenerationError {
    #[error("operating system random source failed: {0}")]
    Random(String),
}

pub fn generate() -> Result<String, KeyGenerationError> {
    let mut bytes = [0_u8; KEY_BYTES];
    getrandom::fill(&mut bytes).map_err(|error| KeyGenerationError::Random(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::generate;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    #[test]
    fn generates_32_bytes_of_unpadded_base64url() {
        let key = generate().expect("OS randomness should be available");
        assert_eq!(URL_SAFE_NO_PAD.decode(&key).unwrap().len(), 32);
        assert!(!key.contains('='));
        assert!(!key.contains([' ', '\n', '\r']));
    }

    #[test]
    fn consecutive_keys_are_different() {
        let first = generate().expect("OS randomness should be available");
        let second = generate().expect("OS randomness should be available");
        assert_ne!(first, second);
    }
}
