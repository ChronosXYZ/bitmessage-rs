use crate::network::node::Node;
use std::error::Error;

mod network;
mod pow;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let mut node = Node::new(None);

    node.run().await;
    Ok(())
}
