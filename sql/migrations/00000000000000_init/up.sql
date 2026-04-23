-- Media
CREATE TABLE media (
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
CREATE TABLE tags (
    -- Relation
    media   BLOB NOT NULL REFERENCES media(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    label   TEXT NOT NULL,
    PRIMARY KEY (media, label)
) STRICT;

-- Activity
CREATE TABLE activity (
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
CREATE TABLE books (
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

-- People
CREATE TABLE people (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())) NOT NULL,
    name    TEXT NOT NULL UNIQUE
) STRICT;

-- Author
CREATE TABLE author (
    -- Relation
    book    BLOB NOT NULL REFERENCES books(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    person  BLOB NOT NULL REFERENCES people(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    -- Sequence
    idx     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (book, person)
) STRICT;

--
-- Links
--
CREATE TABLE links (
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
CREATE TABLE games (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    tgdb    INTEGER UNIQUE,
    title   TEXT NOT NULL,
    -- Platform
    system  TEXT,
    -- Activity
    owned   INTEGER NOT NULL DEFAULT 0,
    rated   INTEGER CHECK(rated BETWEEN 1 AND 5)
) STRICT;

-- Consoles
CREATE TABLE game_consoles (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())),
    title   TEXT NOT NULL,
    -- Platform
    system  TEXT,
    -- Hardware
    model   TEXT,
    revision TEXT,
    serial  TEXT,
    variation TEXT
) STRICT;

-- Releases
CREATE TABLE game_releases (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())),
    title   TEXT NOT NULL,
    -- Platform
    system  TEXT,
    -- Hardware
    model   TEXT,
    revision TEXT
) STRICT;

-- Accessories
CREATE TABLE game_accessories (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())),
    title   TEXT NOT NULL,
    -- Platform
    system  TEXT,
    -- Hardware
    model   TEXT,
    revision TEXT,
    serial  TEXT,
    variation TEXT
) STRICT;

--
-- Films
--
CREATE TABLE films (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    tmdb    INTEGER UNIQUE,
    title   TEXT NOT NULL,
    -- Metadata
    year    INTEGER,
    -- Activity
    rated   INTEGER CHECK(rated BETWEEN 1 AND 5)
) STRICT;

--
-- Shows
--
CREATE TABLE shows (
    -- Identity
    id      BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid()))
        REFERENCES media(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    tmdb    INTEGER UNIQUE,
    title   TEXT NOT NULL,
    -- Metadata
    year    INTEGER,
    -- Activity
    rated   INTEGER CHECK(rated BETWEEN 1 AND 5)
) STRICT;
