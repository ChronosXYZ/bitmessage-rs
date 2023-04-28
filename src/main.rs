use crate::network::node::Node;
use std::error::Error;

mod network;
mod pow;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let node = Node::new();

    node.run().await;
    Ok(())
}
