use std::{collections::HashMap, error::Error, time};

use async_trait::async_trait;

use crate::{network::messages::Object, repositories::inventory::InventoryRepository};

pub struct MemoryInventoryRepository {
    objects: HashMap<Vec<u8>, ObjectWrapper>,
}

struct ObjectWrapper {
    o: Object,
    ttl: time::SystemTime,
}

impl MemoryInventoryRepository {
    pub fn new() -> MemoryInventoryRepository {
        MemoryInventoryRepository {
            objects: HashMap::new(),
        }
    }
}

#[async_trait]
impl InventoryRepository for MemoryInventoryRepository {
    async fn get(&self) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let inv = self.objects.keys().cloned().collect();
        Ok(inv)
    }

    async fn store_object(&mut self, hash: Vec<u8>, o: Object) -> Result<(), Box<dyn Error>> {
        self.objects.insert(
            hash,
            ObjectWrapper {
                o,
                ttl: time::SystemTime::now() + time::Duration::from_secs(3600),
            },
        );
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<usize, Box<dyn Error>> {
        let n = time::SystemTime::now();
        let before_cleanup = self.objects.len();
        self.objects
            .retain(|_, v| if v.ttl < n { false } else { true });
        let after_cleanup = self.objects.len();
        Ok(before_cleanup - after_cleanup)
    }

    async fn get_missing_objects(
        &self,
        hashes: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let mut hashes_cloned = hashes.clone();
        hashes_cloned.retain(|h| match self.objects.get(h) {
            Some(_) => true,
            None => false,
        });
        Ok(hashes_cloned)
    }

    async fn get_object(&self, hash: Vec<u8>) -> Result<Option<Object>, Box<dyn Error>> {
        match self.objects.get(&hash) {
            Some(ow) => Ok(Some(ow.o.clone())),
            None => Ok(None),
        }
    }
}
