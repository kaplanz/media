//! Load subcommand.

import { Database } from "bun:sqlite";

import { BY_KIND, type Kind } from "../models";
import * as db from "../sql";
import * as schema from "../sql/schema";
import { refs } from "../web/routes/games/owned";

import type { Format } from "./dump";

type Fields = Record<string, unknown>;

/** Load arguments. */
export type Args = {
    /** SQLite database file. */
    db: string;
    /** Input format. */
    fmt: Format;
    /** Input file, or stdin when absent. */
    input?: string | undefined;
};

/** Stored media record. */
type Entry = {
    kind: Kind;
    item: Fields & { id: string };
    meta: { created: number; updated: number };
    logs: { id: string; kind: string; date: number }[];
    tags: string[];
};

/**
 * Stored owned game item with its game references.
 *
 * Dumps written before the list was renamed to `games` carry it as `game`, so
 * that field is read as either a lone identifier or the list itself.
 */
type Owned = Fields & {
    id: string;
    game?: string | string[];
    games?: string[];
};

/** Top-level dump payload. */
type Payload = {
    media?: Entry[];
    books?: { owned?: Fields[] };
    games?: { owned?: Owned[] };
};

/** Load entrypoint. */
export async function main(args: Args) {
    // Read input
    const src = await read(args.input);

    // Deserialize collection
    if (args.fmt === "sql") execute(args.db, src);
    else insert(db.open(args.db), JSON.parse(src) as Payload);
}

/** Reads the input file, or stdin when none is given. */
async function read(input: string | undefined) {
    if (input && input !== "-") return Bun.file(input).text();
    return new Response(Bun.stdin.stream()).text();
}

/** Executes a SQL dump against the database. */
function execute(url: string, src: string) {
    // Apply schema
    db.open(url);

    // Execute statements
    const sqlite = new Database(url, { create: true });
    try {
        sqlite.exec(src);
    } finally {
        sqlite.close();
    }
}

/**
 * Inserts a JSON dump into the database.
 *
 * Existing rows are left untouched, so loading the same dump twice is
 * equivalent to loading it once.
 */
function insert(cxn: db.Cxn, payload: Payload) {
    cxn.transaction((tx) => {
        // Insert media records
        for (const entry of payload.media ?? []) {
            const decl = BY_KIND.get(entry.kind);
            if (!decl) continue;
            const { id } = entry.item;

            tx.insert(schema.media)
                .values({
                    id,
                    kind: entry.kind,
                    created: entry.meta.created,
                    updated: entry.meta.updated,
                })
                .onConflictDoNothing()
                .run();

            // Take only the declared columns, since an item also carries the
            // fields assembled from related tables
            const row: Fields = { id };
            for (const name of decl.columns) row[name] = entry.item[name];
            tx.insert(decl.table)
                .values(row as typeof decl.table.$inferInsert)
                .onConflictDoNothing()
                .run();

            const authors = (entry.item.authors ?? []) as string[];
            authors.forEach((name, idx) => {
                tx.insert(schema.books_author)
                    .values({ book: id, name, idx })
                    .onConflictDoNothing()
                    .run();
            });

            for (const label of entry.tags) {
                tx.insert(schema.tags)
                    .values({ media: id, label })
                    .onConflictDoNothing()
                    .run();
            }
            for (const log of entry.logs) {
                tx.insert(schema.logs)
                    .values({
                        id: log.id,
                        media: id,
                        kind: log.kind,
                        date: log.date,
                    })
                    .onConflictDoNothing()
                    .run();
            }
        }

        // Insert owned game items
        for (const entry of payload.games?.owned ?? []) {
            const { game, games, ...row } = entry;
            tx.insert(schema.games_owned)
                .values(row as typeof schema.games_owned.$inferInsert)
                .onConflictDoNothing()
                .run();
            const held = Array.isArray(game) ? game : refs({ game, games });
            (held ?? []).forEach((target, idx) => {
                tx.insert(schema.games_owned_ref)
                    .values({ owned: entry.id, game: target, idx })
                    .onConflictDoNothing()
                    .run();
            });
        }

        // Insert owned book copies
        for (const entry of payload.books?.owned ?? []) {
            tx.insert(schema.books_owned)
                .values(entry as typeof schema.books_owned.$inferInsert)
                .onConflictDoNothing()
                .run();
        }
    });
}
