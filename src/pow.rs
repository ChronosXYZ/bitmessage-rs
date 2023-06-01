use chrono::Utc;
use hmac_sha512::Hash;
use log::info;

use crate::network::messages::Object;

pub(crate) const NETWORK_MIN_NONCE_TRIALS_PER_BYTE: i32 = 1000;
pub(crate) const NETWORK_MIN_EXTRA_BYTES: i32 = 1000;

#[derive(thiserror::Error, Debug)]
pub enum PoWError {
    #[error("proof of work of object is insufficient (trivial_value > target)")]
    InsufficientProofOfWork,
}

/// Function to do PoW for sending messages into network
pub(crate) async fn do_pow(target: u128, initial_hash: Vec<u8>) -> (u128, u128) {
    info!("PoW has started");

    let mut nonce: u128 = 0;
    let mut trial_value = u128::MAX;
    while trial_value > target {
        nonce += 1;
        trial_value = u128::from_be_bytes(
            Hash::hash(Hash::hash(
                [&nonce.to_be_bytes()[..], &initial_hash[..]].concat(),
            ))[0..8]
                .try_into()
                .unwrap(),
        );
    }

    info!("PoW has ended");
    return (trial_value, nonce);
}

/// Function to check if object nonce is properly calculated on sender's side.
pub(crate) fn check_pow(target: u128, nonce: u128, initial_hash: Vec<u8>) -> Result<(), PoWError> {
    let trial_value = u128::from_be_bytes(
        Hash::hash(Hash::hash(
            [&nonce.to_be_bytes()[..], &initial_hash[..]].concat(),
        ))[0..8]
            .try_into()
            .unwrap(),
    );
    if trial_value > target {
        return Err(PoWError::InsufficientProofOfWork);
    }

    Ok(())
}

pub(crate) fn get_pow_target(
    object: &Object,
    mut nonce_trials_per_byte: i32,
    mut extra_bytes: i32,
) -> u128 {
    if nonce_trials_per_byte == 0 {
        nonce_trials_per_byte = NETWORK_MIN_NONCE_TRIALS_PER_BYTE;
    }
    if extra_bytes == 0 {
        extra_bytes = NETWORK_MIN_EXTRA_BYTES;
    }

    let ttl = (object.expires - Utc::now().timestamp()) as u128;
    let payload_bytes =
        (serde_cbor::to_vec(&object.kind).unwrap().len() + (extra_bytes as usize) + 8) as u128;
    let denominator =
        (nonce_trials_per_byte as u128) * (payload_bytes + ((ttl * payload_bytes) / 2_u128.pow(16)));

    2_u128.pow(64) / denominator
}
