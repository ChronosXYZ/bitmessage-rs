use std::error::Error;

use async_trait::async_trait;

use crate::network::messages::Object;

#[async_trait]
pub trait InventoryRepository {
    /// Get current inventory vector
    async fn get(&self) -> Result<Vec<Vec<u8>>, Box<dyn Error>>;

    /// Get object by its hash
    async fn get_object(&self, hash: Vec<u8>) -> Result<Option<Object>, Box<dyn Error>>;

    /// Filter inventory vector with missing objects
    async fn get_missing_objects(
        &self,
        hashes: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn Error>>;

    /// Store received object
    async fn store_object(&mut self, o: Object) -> Result<(), Box<dyn Error>>;

    /// Cleanup the storage of expired items
    async fn cleanup(&self) -> Result<i32, Box<dyn Error>>;
}
