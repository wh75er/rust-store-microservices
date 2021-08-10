-- Your SQL goes here

CREATE TABLE orders
(
    id         SERIAL CONSTRAINT orders_pkey PRIMARY KEY,
    item_uid   UUID         NOT NULL,
    order_date TIMESTAMP    NOT NULL,
    order_uid  UUID         NOT NULL CONSTRAINT idx_orders_order_uid UNIQUE,
    status     VARCHAR(255) NOT NULL,
    user_uid   UUID         NOT NULL
);
