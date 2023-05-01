use std::error::Error;

use async_std::task;
use signal_hook::{consts::SIGTERM, iterator::Signals};

mod network;
mod pow;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let (mut client, worker) = network::new(None);

    task::spawn(worker.run());

    client
        .start_listening("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .await
        .expect("Listening not to fail");

    let mut signals = Signals::new(&[SIGTERM])?;

    for sig in signals.forever() {
        println!("Received signal {:?}", sig);
    }

    Ok(())
}
