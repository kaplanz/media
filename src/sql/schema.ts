//! Database schema.
//!
//! Each table is exported under its own name, in the order it appears in the
//! schema.

import {
    customType,
    integer,
    primaryKey,
    sqliteTable,
    text,
} from "drizzle-orm/sqlite-core";

/** UUIDs are stored as blobs and exposed as hyphenated lowercase text. */
const uuid = customType<{ data: string; driverData: Buffer }>({
    dataType: () => "blob",
    toDriver: (id) => Buffer.from(id.replaceAll("-", ""), "hex"),
    fromDriver: (raw) => {
        const hex = Buffer.from(raw).toString("hex");
        return [
            hex.slice(0, 8),
            hex.slice(8, 12),
            hex.slice(12, 16),
            hex.slice(16, 20),
            hex.slice(20),
        ].join("-");
    },
});

/** Booleans are stored as the integers zero and one. */
const flag = () => integer({ mode: "boolean" }).notNull();

export const media = sqliteTable("media", {
    id: uuid().primaryKey(),
    kind: text().notNull(),
    created: integer().notNull(),
    updated: integer().notNull(),
});

export const tags = sqliteTable(
    "tags",
    {
        media: uuid().notNull(),
        label: text().notNull(),
    },
    (self) => [primaryKey({ columns: [self.media, self.label] })],
);

export const logs = sqliteTable("logs", {
    id: uuid().primaryKey(),
    media: uuid().notNull(),
    kind: text().notNull(),
    date: integer().notNull(),
});

export const books = sqliteTable("books", {
    id: uuid().primaryKey(),
    isbn: text(),
    hcid: integer(),
    title: text().notNull(),
    cover: text(),
    about: text(),
    color: text(),
});

export const books_owned = sqliteTable("books_owned", {
    id: uuid().primaryKey(),
    isbn: text().notNull(),
    title: text(),
    edition: text(),
});

export const books_author = sqliteTable(
    "books_author",
    {
        book: uuid().notNull(),
        name: text().notNull(),
        idx: integer().notNull(),
    },
    (self) => [primaryKey({ columns: [self.book, self.name] })],
);

export const links = sqliteTable("links", {
    id: uuid().primaryKey(),
    url: text().notNull(),
    title: text(),
});

export const games = sqliteTable("games", {
    id: uuid().primaryKey(),
    title: text().notNull(),
    platform: text(),
    rating: integer(),
});

export const games_owned = sqliteTable("games_owned", {
    id: uuid().primaryKey(),
    kind: text().notNull(),
    title: text(),
    platform: text(),
    region: text(),
    model: text(),
    revision: text(),
    serial: text(),
    variant: text(),
    complete: flag(),
    modified: flag(),
});

export const games_owned_ref = sqliteTable(
    "games_owned_ref",
    {
        owned: uuid().notNull(),
        game: uuid().notNull(),
        idx: integer().notNull(),
    },
    (self) => [primaryKey({ columns: [self.owned, self.game] })],
);

export const films = sqliteTable("films", {
    id: uuid().primaryKey(),
    tmdb: integer(),
    title: text().notNull(),
    year: integer(),
    rating: integer(),
});

export const shows = sqliteTable("shows", {
    id: uuid().primaryKey(),
    tmdb: integer(),
    title: text().notNull(),
    year: integer(),
    rating: integer(),
});
