use chrono::Utc;
use num_bigint::BigUint;
use once_cell::sync::Lazy;
use sha2::Digest;
use sha2::Sha512;

use crate::network::messages::Object;

pub mod async_pow;
pub mod sync_pow;

pub const NETWORK_MIN_NONCE_TRIALS_PER_BYTE: i32 = 1000;
pub const NETWORK_MIN_EXTRA_BYTES: i32 = 1000;

#[derive(thiserror::Error, Debug)]
pub enum PoWError {
    #[error("proof of work of object is insufficient (trivial_value > target)")]
    InsufficientProofOfWork,
}

static TWO_POW_16: Lazy<BigUint> = Lazy::new(|| BigUint::from(2 as u32).pow(16));
static TWO_POW_64: Lazy<BigUint> = Lazy::new(|| BigUint::from(2 as u32).pow(64));

/// Function to check if object nonce is properly calculated on sender's side.
pub(crate) fn check_pow(
    target: BigUint,
    nonce: BigUint,
    initial_hash: Vec<u8>,
) -> Result<(), PoWError> {
    let mut hasher = Sha512::new();
    hasher.update(nonce.to_bytes_be());
    hasher.update(initial_hash.as_slice());
    let result_hash = Sha512::digest(&hasher.finalize());
    let trial_value = BigUint::from_bytes_be(&result_hash[0..8]);

    if trial_value > target {
        return Err(PoWError::InsufficientProofOfWork);
    }

    Ok(())
}

pub(crate) fn get_pow_target(
    object: &Object,
    mut nonce_trials_per_byte: i32,
    mut extra_bytes: i32,
) -> BigUint {
    if nonce_trials_per_byte == 0 {
        nonce_trials_per_byte = NETWORK_MIN_NONCE_TRIALS_PER_BYTE;
    }
    if extra_bytes == 0 {
        extra_bytes = NETWORK_MIN_EXTRA_BYTES;
    }

    let ttl = BigUint::from((object.expires - Utc::now().timestamp()) as u64);
    let payload_bytes =
        BigUint::from(serde_cbor::to_vec(&object.kind).unwrap().len() + (extra_bytes as usize) + 8);
    let denominator: BigUint = BigUint::from(nonce_trials_per_byte as u32)
        * (payload_bytes.clone() + ((ttl * payload_bytes) / TWO_POW_16.clone()));

    TWO_POW_64.clone() / denominator
}
