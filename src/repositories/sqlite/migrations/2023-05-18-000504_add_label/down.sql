-- This file should undo anything in `up.sql`
PRAGMA foreign_keys=off;

BEGIN TRANSACTION;

ALTER TABLE addresses RENAME TO _addresses_old;

CREATE TABLE addresses (
    address TEXT PRIMARY KEY NOT NULL,
    tag TEXT NOT NULL,
    public_signing_key BLOB,
    public_encryption_key BLOB,
    private_signing_key BLOB,
    private_encryption_key BLOB
);


INSERT INTO addresses(address, tag, public_signing_key, public_encryption_key, private_signing_key, private_encryption_key)
SELECT address, tag, public_signing_key, public_encryption_key, private_signing_key, private_encryption_key
FROM _addresses_old;

DROP TABLE _addresses_old;

COMMIT;

PRAGMA foreign_keys=on;
