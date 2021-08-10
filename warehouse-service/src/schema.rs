table! {
    items (id) {
        id -> Int4,
        available_count -> Int4,
        model -> Varchar,
        size -> Varchar,
    }
}

table! {
    order_items (id) {
        id -> Int4,
        canceled -> Nullable<Bool>,
        order_item_uid -> Uuid,
        order_uid -> Uuid,
        item_id -> Nullable<Int4>,
    }
}

joinable!(order_items -> items (item_id));

allow_tables_to_appear_in_same_query!(
    items,
    order_items,
);
