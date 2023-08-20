// @generated automatically by Diesel CLI.

diesel::table! {
    addresses (address) {
        address -> Text,
        tag -> Text,
        public_signing_key -> Nullable<Binary>,
        public_encryption_key -> Nullable<Binary>,
        private_signing_key -> Nullable<Binary>,
        private_encryption_key -> Nullable<Binary>,
        label -> Nullable<Text>,
    }
}

diesel::table! {
    inventory (hash) {
        hash -> Text,
        object_type -> Integer,
        nonce -> Binary,
        data -> Binary,
        expires -> Timestamp,
        signature -> Binary,
    }
}

diesel::table! {
    messages (hash) {
        hash -> Text,
        sender -> Text,
        recipient -> Text,
        data -> Binary,
        created_at -> Timestamp,
        status -> Text,
        signature -> Binary,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    addresses,
    inventory,
    messages,
);
