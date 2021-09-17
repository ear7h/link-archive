PRAGMA foreign_keys = ON;

BEGIN EXCLUSIVE;

CREATE TABLE migrations (
	name text NOT NULL UNIQUE,
	created text NOT NULL DEFAULT (datetime('now', 'utc'))
);

INSERT INTO migrations (name) VALUES ('2021-08-29-init.sql');

CREATE TABLE users (
	id integer PRIMARY KEY,
	name text NOT NULL UNIQUE,
	password text NOT NULL,
	token_version integer NOT NULL DEFAULT 0,
	created text NOT NULL DEFAULT (datetime('now', 'utc')),
	deleted text
);

CREATE TABLE links (
	user_id integer NOT NULL REFERENCES users(id),
	url text NOT NULL,
	created text NOT NULL DEFAULT (datetime('now', 'utc')),
	deleted text,
	PRIMARY KEY (user_id, url)
);

END;
