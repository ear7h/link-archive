PRAGMA foreign_keys = ON;

BEGIN EXCLUSIVE;

INSERT INTO migrations (name) VALUES ('2021-09-18-external-password.sql');

ALTER TABLE users DROP COLUMN password;
ALTER TABLE users DROP COLUMN token_version;

END;
