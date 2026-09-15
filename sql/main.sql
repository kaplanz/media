-- Media collection schema.

-- Media
CREATE TABLE IF NOT EXISTS media (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())) NOT NULL,
    kind    TEXT NOT NULL CHECK (
        kind IN ('book', 'film', 'game', 'link', 'show')
    ),
    -- Metadata
    created INTEGER DEFAULT (UNIXEPOCH()) NOT NULL,
    updated INTEGER DEFAULT (UNIXEPOCH()) NOT NULL
) STRICT;

-- Tags
CREATE TABLE IF NOT EXISTS tags (
    -- Relation
    media   BLOB NOT NULL REFERENCES media(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    label   TEXT NOT NULL,
    PRIMARY KEY (media, label)
) STRICT;

-- Logs
CREATE TABLE IF NOT EXISTS logs (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())) NOT NULL,
    media   BLOB NOT NULL REFERENCES media(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    -- Activity
    kind    TEXT NOT NULL CHECK (kind IN ('start', 'stop', 'done')),
    date    INTEGER NOT NULL
) STRICT;

--
-- Books
--
CREATE TABLE IF NOT EXISTS books (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    isbn    TEXT UNIQUE CHECK(isbn IS NULL OR length(isbn) = 13),
    hcid    INTEGER UNIQUE,
    title   TEXT NOT NULL,
    -- Property
    cover   TEXT,
    about   TEXT,
    color   TEXT
) STRICT;

-- Owned
CREATE TABLE IF NOT EXISTS books_owned (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())) NOT NULL,
    isbn    TEXT NOT NULL CHECK(length(isbn) = 13),
    title   TEXT,
    -- Edition
    edition TEXT
) STRICT;

CREATE INDEX IF NOT EXISTS books_owned_isbn ON books_owned(isbn);

-- Author
CREATE TABLE IF NOT EXISTS books_author (
    -- Relation
    book    BLOB NOT NULL REFERENCES books(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    name    TEXT NOT NULL,
    -- Sequence
    idx     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (book, name)
) STRICT;

--
-- Links
--
CREATE TABLE IF NOT EXISTS links (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    -- Metadata
    url     TEXT NOT NULL,
    title   TEXT
) STRICT;

--
-- Games
--
CREATE TABLE IF NOT EXISTS games (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    title   TEXT NOT NULL,
    -- Platform
    platform TEXT,
    -- Activity
    rating  INTEGER CHECK(rating BETWEEN 1 AND 5)
) STRICT;

-- Owned
CREATE TABLE IF NOT EXISTS games_owned (
    -- Identity
    id       BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())) NOT NULL,
    kind     TEXT NOT NULL CHECK (
        kind IN ('release', 'console', 'extra')
    ),
    title    TEXT,
    -- Platform
    platform TEXT,
    region   TEXT CHECK(region IS NULL OR length(region) = 2),
    -- Hardware
    model    TEXT,
    revision TEXT,
    serial   TEXT,
    variant  TEXT,
    -- Collection
    complete INTEGER NOT NULL DEFAULT 0 CHECK(complete IN (0, 1)),
    modified INTEGER NOT NULL DEFAULT 0 CHECK(modified IN (0, 1))
) STRICT;

-- Reference
CREATE TABLE IF NOT EXISTS games_owned_ref (
    -- Relation
    owned    BLOB NOT NULL REFERENCES games_owned(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    game     BLOB NOT NULL REFERENCES games(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    -- Sequence
    idx      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owned, game)
) STRICT;

-- ROM
CREATE TABLE IF NOT EXISTS games_owned_rom (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())) NOT NULL,
    owned   BLOB NOT NULL REFERENCES games_owned(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    -- Content
    name    TEXT,
    size    INTEGER NOT NULL CHECK(size >= 0),
    -- Digest
    crc32   BLOB NOT NULL CHECK(length(crc32)  = 4),
    md5     BLOB NOT NULL CHECK(length(md5)    = 16),
    sha1    BLOB NOT NULL CHECK(length(sha1)   = 20),
    sha256  BLOB NOT NULL CHECK(length(sha256) = 32),
    -- Metadata
    dumped  INTEGER DEFAULT (UNIXEPOCH()) NOT NULL,
    -- Sequence
    idx     INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE INDEX IF NOT EXISTS games_owned_rom_owned ON games_owned_rom(owned);
CREATE INDEX IF NOT EXISTS games_owned_rom_sha1  ON games_owned_rom(sha1);

--
-- Films
--
CREATE TABLE IF NOT EXISTS films (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    tmdb    INTEGER UNIQUE,
    title   TEXT NOT NULL,
    -- Metadata
    year    INTEGER,
    -- Activity
    rating  INTEGER CHECK(rating BETWEEN 1 AND 5)
) STRICT;

--
-- Shows
--
CREATE TABLE IF NOT EXISTS shows (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    tmdb    INTEGER UNIQUE,
    title   TEXT NOT NULL,
    -- Metadata
    year    INTEGER,
    -- Activity
    rating  INTEGER CHECK(rating BETWEEN 1 AND 5)
) STRICT;
