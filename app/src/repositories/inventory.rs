use std::error::Error;

use async_trait::async_trait;

use crate::network::messages::Object;

#[async_trait]
pub trait InventoryRepository {
    /// Get current inventory vector
    async fn get(&self) -> Result<Vec<String>, Box<dyn Error>>;

    /// Get object by its hash
    async fn get_object(&self, hash: String) -> Result<Option<Object>, Box<dyn Error>>;

    /// Filter inventory vector with missing objects
    async fn get_missing_objects(&self, hashes: &mut Vec<String>) -> Result<(), Box<dyn Error>>;

    /// Store received object
    async fn store_object(&mut self, hash: String, o: Object) -> Result<(), Box<dyn Error>>;

    /// Cleanup the storage of expired items
    async fn cleanup(&mut self) -> Result<usize, Box<dyn Error>>;
}

pub type InventoryRepositorySync = dyn InventoryRepository + Send + Sync;
