BEGIN;

PRAGMA foreign_keys = ON;

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
