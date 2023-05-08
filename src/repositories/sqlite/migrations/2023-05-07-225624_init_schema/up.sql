-- Your SQL goes here

CREATE TABLE addresses (
    address TEXT PRIMARY KEY NOT NULL,
    tag TEXT NOT NULL,
    public_signing_key BLOB,
    public_encryption_key BLOB,
    private_signing_key BLOB,
    private_encryption_key BLOB
);

CREATE TABLE inventory (
    hash TEXT PRIMARY KEY NOT NULL,
    object_type INTEGER NOT NULL,
    nonce BLOB NOT NULL,
    data BLOB NOT NULL,
    expires TIMESTAMP NOT NULL
);

CREATE TABLE messages (
    hash TEXT PRIMARY KEY NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    data BLOB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    status TEXT NOT NULL,
    signature BLOB NOT NULL
)
