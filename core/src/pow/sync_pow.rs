use log::info;
use num_bigint::BigUint;
use sha2::{Digest, Sha512};

/// Function to do PoW for sending messages into network
pub(crate) async fn do_pow(target: BigUint, initial_hash: Vec<u8>) -> (BigUint, BigUint) {
    info!("PoW has started");

    let mut nonce: BigUint = BigUint::from(0u32);
    let mut trial_value = BigUint::parse_bytes(b"99999999999999999999", 10).unwrap();
    while trial_value > target {
        let mut hasher = Sha512::new();
        nonce += 1u32;
        hasher.update(nonce.to_bytes_be());
        hasher.update(initial_hash.as_slice());
        let result_hash = Sha512::digest(&hasher.finalize());
        trial_value = BigUint::from_bytes_be(&result_hash[0..8]);
    }

    info!("PoW has ended");
    return (trial_value, nonce);
}
