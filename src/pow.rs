use bytes::Bytes;
use hmac_sha512::Hash;
use log::info;

#[derive(thiserror::Error, Debug)]
pub enum PoWError {
    #[error("proof of work of object is insufficient (trivial_value > target)")]
    InsufficientProofOfWork,
}

// Function to do PoW for sending messages into network
fn do_pow(target: u64, initial_hash: Bytes) -> (u64, u64) {
    info!("PoW has started");

    let mut nonce: u64 = 0;
    let mut trial_value = u64::MAX;
    while trial_value > target {
        nonce += 1;
        trial_value = u64::from_be_bytes(
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

fn check_pow(target: u64, nonce: u64, initial_hash: Bytes) -> Result<(), PoWError> {
    let trial_value = u64::from_be_bytes(
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
