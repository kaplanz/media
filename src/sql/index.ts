//! Database utilities.

import { Database } from "bun:sqlite";

import { eq } from "drizzle-orm";
import { drizzle } from "drizzle-orm/bun-sqlite";

import ddl from "./main.sql" with { type: "text" };
import * as schema from "./schema";

/** An open database connection. */
export type Cxn = ReturnType<typeof open>;

/**
 * Opens the database at the given path.
 *
 * The schema is applied to an empty database, so a new file needs no separate
 * setup step.
 */
export function open(url: string): ReturnType<typeof drizzle> {
    // Connect to database
    const sqlite = new Database(url, { create: true, strict: false });
    sqlite.exec("PRAGMA foreign_keys = ON");
    sqlite.exec("PRAGMA journal_mode = WAL");

    // Apply schema
    if (!ready(sqlite)) sqlite.exec(ddl);

    return drizzle(sqlite);
}

/** Reports whether the schema has already been applied. */
const ready = (sqlite: Database) =>
    sqlite
        .query("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")
        .get("media") !== null;

/**
 * Returns the number of rows changed by a mutation.
 *
 * The bun-sqlite driver reports this as `changes`, which the shared drizzle
 * result type does not surface.
 */
export const affected = (res: unknown) => (res as { changes: number }).changes;

/** Returns the current Unix timestamp in seconds. */
export const timestamp = () => Math.floor(Date.now() / 1000);

/** Marks a media item as updated. */
export async function touch(cxn: Cxn, id: string) {
    await cxn
        .update(schema.media)
        .set({ updated: timestamp() })
        .where(eq(schema.media.id, id));
}
