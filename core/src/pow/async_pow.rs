use async_std::task;
use futures::{
    channel::{mpsc, oneshot},
    select, FutureExt, SinkExt, StreamExt,
};
use log::info;
use num_bigint::BigUint;
use sha2::{Digest, Sha512};

pub struct AsyncPoW {}

impl AsyncPoW {
    pub fn do_pow(target: BigUint, initial_hash: Vec<u8>) -> oneshot::Receiver<(BigUint, BigUint)> {
        let (mut sender, receiver) = oneshot::channel();
        let (internal_sender, mut internal_receiver) = mpsc::channel(1);

        let mut workers = Vec::new();
        let num_of_cores = num_cpus::get(); // TODO make this setting configurable

        for i in 0..num_of_cores {
            let t = target.clone();
            let ih = initial_hash.clone();
            let mut s = internal_sender.clone();
            let (term_tx, mut term_rx) = oneshot::channel();
            task::spawn_blocking(move || {
                info!("PoW has started");

                let mut nonce: BigUint = BigUint::from(i);
                let mut trial_value = BigUint::parse_bytes(b"99999999999999999999", 10).unwrap();
                while trial_value > t && !term_rx.try_recv().is_err() {
                    nonce += num_of_cores;
                    let result_hash = Sha512::digest(Sha512::digest(
                        [nonce.to_bytes_be().as_slice(), ih.as_slice()].concat(),
                    ));
                    trial_value = BigUint::from_bytes_be(&result_hash[0..8]);
                }

                if !term_rx.try_recv().is_err() {
                    task::block_on(s.send((trial_value, nonce))).unwrap();
                }

                info!("PoW has ended");
            });
            workers.push(term_tx);
        }

        task::spawn(async move {
            let mut cancellation_task = sender.cancellation().fuse();
            select! {
                () = cancellation_task => {
                    log::debug!("cancelling workers");
                    for w in workers.into_iter() {
                        _ = w.send(());
                    }
                    internal_receiver.close();
                    return;
                },
                result = internal_receiver.next() => {
                    if let Some(res) = result {
                        log::debug!("cancelling workers");
                        for w in workers.into_iter() {
                            _ = w.send(());
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
