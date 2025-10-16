use rand::Rng;

pub const DEFAULT_USERNAME: &str = "chisel";

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                        abcdefghijklmnopqrstuvwxyz\
                        0123456789-_/()*&#@";
/// Generates a random password of the specified length.
///
/// # Arguments
///
/// * `length` - The length of the password to generate.
///
/// # Returns
///
/// A randomly generated password as a `String`.
pub fn generate_password(length: usize) -> String {
    let mut rng = rand::rng();

    let password: String = (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    password
}
