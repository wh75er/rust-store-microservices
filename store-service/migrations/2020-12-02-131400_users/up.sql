-- Your SQL goes here

CREATE TABLE users
(
    id       SERIAL CONSTRAINT users_pkey PRIMARY KEY,
    name     VARCHAR(255) NOT NULL CONSTRAINT idx_user_name UNIQUE,
    user_uid UUID         NOT NULL CONSTRAINT idx_user_user_uid UNIQUE
);

INSERT INTO users (name, user_uid)
  VALUES ('Alex', '6d2cb5a0-943c-4b96-9aa6-89eac7bdfd2b');
