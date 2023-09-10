use std::error::Error;

use async_trait::async_trait;
use dyn_clone::{clone_trait_object, DynClone};
use ecies::PublicKey;

use crate::network::address::Address;

#[async_trait]
pub trait AddressRepository: DynClone {
    /// Store known address
    async fn store(&mut self, a: Address) -> Result<(), Box<dyn Error>>;

    /// Delete address from repository
    async fn delete_address(&mut self, ripe: String) -> Result<(), Box<dyn Error>>;

    /// Get address by its ripe hash or tag
    async fn get_by_ripe_or_tag(&self, hash: String) -> Result<Option<Address>, Box<dyn Error>>;

    /// Get contacts with known pubkeys
    async fn get_contacts(&self) -> Result<Vec<Address>, Box<dyn Error>>;

    /// Get own identities, i.e. addresses which have private key
    async fn get_identities(&self) -> Result<Vec<Address>, Box<dyn Error>>;

    async fn update_public_keys(
        &mut self,
        hash: String,
        public_signing_key: PublicKey,
        public_encryption_key: PublicKey,
    ) -> Result<(), Box<dyn Error>>;

    async fn update_label(&mut self, ripe: String, new_label: String)
        -> Result<(), Box<dyn Error>>;
}

clone_trait_object!(AddressRepository);

pub type AddressRepositorySync = dyn AddressRepository + Send + Sync;
