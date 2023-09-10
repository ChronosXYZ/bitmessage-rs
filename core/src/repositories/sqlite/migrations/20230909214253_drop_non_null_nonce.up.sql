-- Add up migration script here
PRAGMA foreign_keys=off;

ALTER TABLE inventory RENAME TO _inventory_old;

CREATE TABLE inventory (
    hash TEXT PRIMARY KEY NOT NULL,
    object_type INTEGER NOT NULL,
    nonce BLOB,
    data BLOB NOT NULL,
    expires TIMESTAMP NOT NULL,
    signature BLOB NOT NULL
);


INSERT INTO inventory(hash, object_type, nonce, data, expires, signature)
SELECT hash, object_type, nonce, data, expires, signature
FROM _inventory_old;

DROP TABLE _inventory_old;

PRAGMA foreign_keys=on;
