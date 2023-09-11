use std::error::Error;

use async_trait::async_trait;
use dyn_clone::{clone_trait_object, DynClone};

use crate::network::messages::Object;

#[async_trait]
pub trait InventoryRepository: DynClone {
    /// Get current inventory vector
    async fn get(&self) -> Result<Vec<String>, Box<dyn Error>>;

    /// Get object by its hash
    async fn get_object(&self, hash: String) -> Result<Option<Object>, Box<dyn Error>>;

    /// Filter inventory vector with missing objects
    async fn get_missing_objects(&self, hashes: Vec<String>)
        -> Result<Vec<String>, Box<dyn Error>>;

    /// Store received object
    async fn store_object(&mut self, o: Object) -> Result<(), Box<dyn Error>>;

    /// Get objects with incomplete proof of work
    async fn get_missing_pow_objects(&self) -> Result<Vec<Object>, Box<dyn Error>>;

    /// Update object nonce when PoW is done
    async fn update_nonce(&mut self, hash: String, nonce: Vec<u8>) -> Result<(), Box<dyn Error>>;

    /// Cleanup the storage of expired items
    async fn cleanup(&mut self) -> Result<usize, Box<dyn Error>>;
}

clone_trait_object!(InventoryRepository);

pub type InventoryRepositorySync = dyn InventoryRepository + Send + Sync;
