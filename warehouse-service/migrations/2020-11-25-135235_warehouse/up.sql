-- Your SQL goes here

CREATE TABLE items
(
  id SERIAL CONSTRAINT items_pkey PRIMARY KEY,
  available_count INT NOT NULL,
  model VARCHAR(255) NOT NULL,
  size VARCHAR(255) NOT NULL
);

INSERT INTO items (available_count, model, size)
  VALUES (10000, 'Lego 8070', 'M');

INSERT INTO items (available_count, model, size)
  VALUES (10000, 'Lego 42070', 'L');

INSERT INTO items (available_count, model, size)
  VALUES (10000, 'Lego 8880', 'L');

CREATE TABLE order_items
(
  id SERIAL CONSTRAINT order_item_pkey PRIMARY KEY,
  canceled BOOLEAN,
  order_item_uid UUID NOT NULL CONSTRAINT idx_order_item_uid UNIQUE,
  order_uid UUID NOT NULL,
  item_id INT CONSTRAINT fk_order_item_id REFERENCES items
);
