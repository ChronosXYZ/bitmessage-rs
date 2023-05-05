use std::error::Error;

use async_trait::async_trait;

use crate::network::address::Address;

#[async_trait]
pub trait AddressRepository {
    /// Store known address
    async fn store(&mut self, a: Address) -> Result<(), Box<dyn Error>>;

    /// Get address by its hash
    async fn get(&self, hash: Vec<u8>) -> Result<Option<Address>, Box<dyn Error>>;

    /// Get contacts with known pubkeys
    async fn get_contacts(&self) -> Result<Vec<Address>, Box<dyn Error>>;

    /// Get own identities, i.e. addresses which have private key
    async fn get_identities(&self) -> Result<Vec<Address>, Box<dyn Error>>;
}
