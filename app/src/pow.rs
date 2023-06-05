use async_std::task::{self, JoinHandle};
use chrono::Utc;
use futures::{
    channel::{mpsc, oneshot},
    select, FutureExt, SinkExt, StreamExt,
};
use log::info;
use num_bigint::BigUint;
use once_cell::sync::Lazy;
use sha2::{Digest, Sha512};

use crate::network::messages::Object;

pub(crate) const NETWORK_MIN_NONCE_TRIALS_PER_BYTE: i32 = 1000;
pub(crate) const NETWORK_MIN_EXTRA_BYTES: i32 = 1000;

static TWO_POW_16: Lazy<BigUint> = Lazy::new(|| BigUint::from(2 as u32).pow(16));
static TWO_POW_64: Lazy<BigUint> = Lazy::new(|| BigUint::from(2 as u32).pow(64));

#[derive(thiserror::Error, Debug)]
pub enum PoWError {
    #[error("proof of work of object is insufficient (trivial_value > target)")]
    InsufficientProofOfWork,
}

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

pub struct AsyncPoW {
    workers: Vec<JoinHandle<()>>,
}

impl AsyncPoW {
    pub fn do_pow(target: BigUint, initial_hash: Vec<u8>) -> oneshot::Receiver<(BigUint, BigUint)> {
        let (mut sender, receiver) = oneshot::channel();
        let (internal_sender, mut internal_receiver) = mpsc::channel(1);

        let mut workers = Vec::new();
        let num_of_cores = num_cpus::get_physical();

        for i in 0..num_of_cores {
            let t = target.clone();
            let ih = initial_hash.clone();
            let mut s = internal_sender.clone();
            let (term_tx, mut term_rx) = oneshot::channel();
            task::spawn(async move {
                info!("PoW has started");

                let mut nonce: BigUint = BigUint::from(i);
                let mut trial_value = BigUint::parse_bytes(b"99999999999999999999", 10).unwrap();
                while trial_value > t && !term_rx.try_recv().is_err() {
                    let mut hasher = Sha512::new();
                    nonce += num_of_cores;
                    hasher.update(nonce.to_bytes_be());
                    hasher.update(ih.as_slice());
                    let result_hash = Sha512::digest(&hasher.finalize());
                    trial_value = BigUint::from_bytes_be(&result_hash[0..8]);
                }

                if !term_rx.try_recv().is_err() {
                    s.send((trial_value, nonce)).await.unwrap();
                }

                info!("PoW has ended");
            });
            workers.push(term_tx);
        }

        task::spawn(async move {
            let mut cancellation_task = sender.cancellation().fuse();
            select! {
                () = cancellation_task => {
                    for w in workers.into_iter() {
                        w.send(());
                    }
                    internal_receiver.close();
                    return;
                },
                result = internal_receiver.next() => {
                    if let Some(res) = result {
                        log::debug!("cancelling workers");
                        for w in workers.into_iter() {
                            w.send(());
                        }
                        sender.send(res).expect("receiver not to be dropped");
                        internal_receiver.close();
                    }
                }
            }
        });
        receiver
    }
}
