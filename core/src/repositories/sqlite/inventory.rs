use crate::network::messages::{Object, ObjectKind};
use crate::pow;
use std::error::Error;

use async_std::task;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::SqlitePool;

use crate::repositories::inventory::InventoryRepository;

use super::models::{self};

#[derive(Clone)]
pub struct SqliteInventoryRepository {
    connection_pool: SqlitePool,
}

impl SqliteInventoryRepository {
    pub fn new(conn_pool: SqlitePool) -> SqliteInventoryRepository {
        SqliteInventoryRepository {
            connection_pool: conn_pool,
        }
    }
}

#[async_trait]
impl InventoryRepository for SqliteInventoryRepository {
    async fn get(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT hash FROM inventory WHERE expires > ?")
                .bind(Utc::now())
                .fetch_all(&self.connection_pool)
                .await?;
        Ok(rows)
    }

    async fn get_object(&self, hash: String) -> Result<Option<Object>, Box<dyn Error>> {
        let obj: Result<models::Object, sqlx::Error> =
            sqlx::query_as("SELECT * FROM inventory WHERE hash = ?")
                .bind(&hash)
                .fetch_one(&self.connection_pool)
                .await;

        if obj.is_err() {
            match obj {
                Err(sqlx::Error::RowNotFound) => return Ok(None),
                _ => return Err(Box::new(obj.err().unwrap())),
            }
        }

        let obj = obj.unwrap();

        let kind: ObjectKind = serde_cbor::from_slice(&obj.data).expect("data not to be malformed");

        Ok(Some(Object {
            hash: bs58::decode(hash).into_vec().unwrap(),
            nonce: obj.nonce.clone(),
            expires: obj.expires.timestamp(),
            kind,
            signature: obj.signature.clone(),
            nonce_trials_per_byte: pow::NETWORK_MIN_NONCE_TRIALS_PER_BYTE, // FIXME save this in db
            extra_bytes: pow::NETWORK_MIN_EXTRA_BYTES,                     // FIXME save this in db
        }))
    }

    /// Filter inventory vector with missing objects
    async fn get_missing_objects(&self, hashes: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
        // delete items from the vector if they are in database
        hashes.retain(|h| {
            let res = task::block_on(
                sqlx::query("SELECT hash FROM inventory WHERE hash = ?")
                    .bind(h)
                    .fetch_one(&self.connection_pool),
            );
            match res {
                Ok(_) => false,
                Err(err) => match err {
                    sqlx::Error::RowNotFound => true,
                    _ => panic!("{}", err),
                },
            }
        });
        Ok(())
    }

    /// Store received object
    async fn store_object(&mut self, hash: String, o: Object) -> Result<(), Box<dyn Error>> {
        let data = serde_cbor::to_vec(&o.kind).expect("data not to be malformed");

        let model = models::Object {
            hash,
            nonce: o.nonce,
            object_type: o.kind.object_type() as i32,
            data,
            expires: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(o.expires, 0).unwrap(),
                Utc,
            ),
            signature: o.signature,
        };

        sqlx::query("INSERT INTO inventory (hash, nonce, object_type, data, expires, signature) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind(model.hash)
            .bind(model.nonce)
            .bind(model.object_type)
            .bind(model.data)
            .bind(model.expires)
            .bind(model.signature).execute(&self.connection_pool).await?;

        Ok(())
    }

    /// Cleanup the storage of expired items
    async fn cleanup(&mut self) -> Result<usize, Box<dyn Error>> {
        let result = sqlx::query("DELETE FROM inventory WHERE expires <= ?")
            .bind(Utc::now())
            .execute(&self.connection_pool)
            .await?;
        Ok(result.rows_affected() as usize)
    }
}
