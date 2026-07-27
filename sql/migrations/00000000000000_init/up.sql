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

-- Logs
CREATE TABLE logs (
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

-- Author
CREATE TABLE books_author (
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
    title   TEXT NOT NULL,
    -- Platform
    system  TEXT,
    -- Activity
    rating  INTEGER CHECK(rating BETWEEN 1 AND 5)
) STRICT;

-- Owned
CREATE TABLE games_owned (
    -- Identity
    id       BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())),
    game     BLOB NOT NULL REFERENCES games(id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    -- Platform
    system   TEXT,
    region   TEXT CHECK(region IS NULL OR length(region) = 2),
    -- Hardware
    model    TEXT,
    revision TEXT,
    serial   TEXT,
    -- Collection
    complete INTEGER NOT NULL DEFAULT 0 CHECK(complete IN (0, 1)),
    modified INTEGER NOT NULL DEFAULT 0 CHECK(modified IN (0, 1))
) STRICT;

-- Systems
CREATE TABLE games_system (
    -- Identity
    id       BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())),
    title    TEXT NOT NULL,
    -- Platform
    system   TEXT,
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

-- Extras
CREATE TABLE games_extras (
    -- Identity
    id       BLOB PRIMARY KEY DEFAULT (uuid_blob(uuid())),
    title    TEXT NOT NULL,
    -- Platform
    system   TEXT,
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
    rating  INTEGER CHECK(rating BETWEEN 1 AND 5)
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
    rating  INTEGER CHECK(rating BETWEEN 1 AND 5)
) STRICT;
