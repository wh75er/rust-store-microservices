table! {
    warranty (id) {
        id -> Int4,
        comment -> Nullable<Varchar>,
        item_uid -> Uuid,
        status -> Varchar,
        warranty_date -> Timestamp,
    }
}
