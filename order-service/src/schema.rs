table! {
    orders (id) {
        id -> Int4,
        item_uid -> Uuid,
        order_date -> Timestamp,
        order_uid -> Uuid,
        status -> Varchar,
        user_uid -> Uuid,
    }
}
