use rand_core::CryptoRngCore;
use reticulum::crypt::fernet::{Fernet, Token};
use reticulum::error::RnsError;
use reticulum::identity::{DerivedKey, PrivateIdentity, PUBLIC_KEY_LENGTH};
use x25519_dalek::{PublicKey, StaticSecret};

pub fn encrypt_for_public_key<R: CryptoRngCore + Copy>(
    public_key: &PublicKey,
    salt: &[u8],
    plaintext: &[u8],
    rng: R,
) -> Result<Vec<u8>, RnsError> {
    reticulum::ratchets::encrypt_for_public_key(public_key, salt, plaintext, rng)
}

pub fn decrypt_with_private_key(
    private_key: &StaticSecret,
    salt: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, RnsError> {
    reticulum::ratchets::decrypt_with_private_key(private_key, salt, ciphertext)
}

pub fn decrypt_with_identity(
    identity: &PrivateIdentity,
    salt: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, RnsError> {
    if ciphertext.len() <= PUBLIC_KEY_LENGTH {
        return Err(RnsError::InvalidArgument);
    }
    let mut pub_bytes = [0u8; PUBLIC_KEY_LENGTH];
    pub_bytes.copy_from_slice(&ciphertext[..PUBLIC_KEY_LENGTH]);
    let ephemeral_public = PublicKey::from(pub_bytes);
    let derived: DerivedKey = identity.derive_key(&ephemeral_public, Some(salt));
    let key_bytes = derived.as_bytes();
    let split = key_bytes.len() / 2;

    let fernet =
        Fernet::new_from_slices(&key_bytes[..split], &key_bytes[split..], rand_core::OsRng);
    let token = Token::from(&ciphertext[PUBLIC_KEY_LENGTH..]);
    let verified = fernet.verify(token)?;
    let mut out = vec![0u8; ciphertext.len()];
    let plain = fernet.decrypt(verified, &mut out)?;
    Ok(plain.as_bytes().to_vec())
}
