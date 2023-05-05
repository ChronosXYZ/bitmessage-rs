use std::error::Error;

use async_trait::async_trait;

use crate::{
    network::messages::{InventoryVector, Object},
    repositories::inventory::InventoryRepository,
};

pub struct MemoryInventoryRepository {}

impl MemoryInventoryRepository {
    pub fn new() -> MemoryInventoryRepository {
        MemoryInventoryRepository {}
    }
}

#[async_trait]
impl InventoryRepository for MemoryInventoryRepository {
    async fn get(&self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        todo!()
    }
    async fn store_object(&mut self, o: Object) -> Result<(), Box<dyn Error>> {
        todo!()
    }
    async fn cleanup(&self) -> Result<i32, Box<dyn Error>> {
        todo!()
    }

    async fn get_missing_objects(
        &self,
        hashes: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        todo!()
    }

    async fn get_object(&self, hash: Vec<u8>) -> Result<Option<Object>, Box<dyn Error>> {
        todo!()
    }
}
