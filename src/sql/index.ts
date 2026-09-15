//! Database utilities.

import { Database } from "bun:sqlite";

import { eq } from "drizzle-orm";
import { drizzle } from "drizzle-orm/bun-sqlite";

import ddl from "../../sql/main.sql" with { type: "text" };
import * as schema from "./schema";

/** An open database connection. */
export type Cxn = ReturnType<typeof open>;

/**
 * Opens the database at the given path.
 *
 * The schema is applied in full every time, so a new file needs no separate
 * setup step and an existing one picks up whatever has been added since it was
 * written. Every statement is idempotent, which is what makes that safe.
 */
export function open(url: string): ReturnType<typeof drizzle> {
    // Connect to database
    const sqlite = new Database(url, { create: true, strict: false });
    sqlite.exec("PRAGMA foreign_keys = ON");
    sqlite.exec("PRAGMA journal_mode = WAL");

    // Apply schema
    sqlite.exec(ddl);

    return drizzle(sqlite);
}

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
