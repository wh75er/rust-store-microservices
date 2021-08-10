-- Your SQL goes here

CREATE TABLE warranty (
  id SERIAL CONSTRAINT warranty_pkey PRIMARY KEY,
  comment VARCHAR(1024),
  item_uid UUID NOT NULL CONSTRAINT idx_warranty_item_uid UNIQUE,
  status VARCHAR(255) NOT NULL,
  warranty_date TIMESTAMP NOT NULL
);
