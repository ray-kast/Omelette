CREATE TABLE form_ids (
  norm TEXT PRIMARY KEY NOT NULL,
  id   INTEGER NOT NULL
);

CREATE TABLE forms (
  oid   INTEGER PRIMARY KEY,
  id    INTEGER NOT NULL,
  blank TEXT NOT NULL,
  full  TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE set_ids (
  key TEXT PRIMARY KEY NOT NULL,
  id  INTEGER NOT NULL
);

CREATE TABLE sets (
  oid  INTEGER PRIMARY KEY,
  id   INTEGER NOT NULL,
  norm TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE set_keys (
  oid INTEGER PRIMARY KEY,
  len INTEGER NOT NULL,
  key TEXT NOT NULL
) WITHOUT ROWID;