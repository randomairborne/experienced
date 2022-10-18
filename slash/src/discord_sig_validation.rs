use ed25519_dalek::{PublicKey, Signature, Verifier};

fn validate_discord_sig(
    headers: &http::header::HeaderMap,
    body: &[u8],
    pub_key_string: String,
) -> std::result::Result<(), SignatureValidationError> {
    let sig_hex = hex::decode(
        headers
            .get("X-Signature-Ed25519")
            .ok_or(SignatureValidationError::MissingSignatureHeader)?,
    )?;
    let mut sig_arr: [u8; 64] = [0; 64];
    for (i, byte) in sig_hex.into_iter().enumerate() {
        sig_arr[i] = byte;
    }
    let sig = Signature::from_bytes(&sig_arr)?;
    let timestamp = headers
        .get("X-Signature-Timestamp")
        .ok_or(SignatureValidationError::MissingTimestampHeader)?;
    let pub_key = PublicKey::from_bytes(&hex::decode(pub_key_string)?)?;
    let to_be_verified: Vec<u8> = timestamp
        .as_bytes()
        .iter()
        .chain(body.iter())
        .cloned()
        .collect();
    pub_key.verify(to_be_verified.as_slice(), &sig)?;
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum SignatureValidationError {
    #[error("ed25519-dalek signature error")]
    Dalek(#[from] ed25519_dalek::SignatureError),
    #[error("Hex decode error")]
    HexDecode(#[from] hex::FromHexError),
    #[error("Missing X-Signature-Ed25519 header")]
    MissingSignatureHeader,
    #[error("Missing X-Signature-Timestamp header")]
    MissingTimestampHeader,
}
