//! Record assembly.

import { and, eq, inArray } from "drizzle-orm";

import type { Kind } from "../models";
import type { Cxn } from "../sql";
import * as schema from "../sql/schema";

/** Activity log. */
export type Log = { id: string; kind: string; date: number };

/** Item metadata. */
export type Meta = { created: number; updated: number };

/** Media record. */
export type Record<T> = {
    kind: Kind;
    item: T;
    meta: Meta;
    logs: Log[];
    tags: string[];
};

/** Groups values by key, preserving the order they were loaded in. */
function group<V>(rows: { key: string; value: V }[]) {
    const out = new Map<string, V[]>();
    for (const { key, value } of rows) {
        const held = out.get(key);
        if (held) held.push(value);
        else out.set(key, [value]);
    }
    return out;
}

/** Loads tags for a set of media IDs, grouped by ID. */
export async function tags(cxn: Cxn, ids: string[]) {
    const rows = await cxn
        .select({ key: schema.tags.media, value: schema.tags.label })
        .from(schema.tags)
        .where(inArray(schema.tags.media, ids))
        .orderBy(schema.tags.label);
    return group(rows);
}

/** Loads authors for a set of book IDs, grouped by ID and in listed order. */
export async function authors(cxn: Cxn, ids: string[]) {
    const rows = await cxn
        .select({
            key: schema.books_author.book,
            value: schema.books_author.name,
        })
        .from(schema.books_author)
        .where(inArray(schema.books_author.book, ids))
        .orderBy(schema.books_author.idx);
    return group(rows);
}

/**
 * Attaches the item fields a kind assembles from related tables.
 *
 * Only books carry one: the ordered author list, which lives in its own table
 * rather than as a column, so it is filled in once the items are loaded.
 */
export async function extend(
    cxn: Cxn,
    entries: { kind: Kind; item: { id: string } }[],
) {
    const books = entries.filter((entry) => entry.kind === "book");
    if (!books.length) return;
    const names = await authors(
        cxn,
        books.map((entry) => entry.item.id),
    );
    for (const entry of books) {
        const item = entry.item as { id: string; authors?: string[] };
        item.authors = names.get(item.id) ?? [];
    }
}

/** Loads logs for a set of media IDs, grouped by ID. */
export async function logs(cxn: Cxn, ids: string[]) {
    const rows = await cxn
        .select({
            key: schema.logs.media,
            value: {
                id: schema.logs.id,
                kind: schema.logs.kind,
                date: schema.logs.date,
            },
        })
        .from(schema.logs)
        .where(inArray(schema.logs.media, ids))
        .orderBy(schema.logs.date);
    return group<Log>(rows);
}

/** Wraps items in the record envelope, attaching their tags and logs. */
export async function wrap<T extends { id: string }>(
    cxn: Cxn,
    kind: Kind,
    rows: { item: T; created: number; updated: number }[],
): Promise<Record<T>[]> {
    const ids = rows.map((row) => row.item.id);
    const [applied, activity] = await Promise.all([
        tags(cxn, ids),
        logs(cxn, ids),
    ]);
    const out = rows.map(({ item, created, updated }) => ({
        kind,
        item,
        meta: { created, updated },
        logs: activity.get(item.id) ?? [],
        tags: applied.get(item.id) ?? [],
    }));
    await extend(cxn, out);
    return out;
}

/** Reports whether a media item exists, optionally constrained to one kind. */
export async function exists(cxn: Cxn, id: string, kind?: Kind) {
    const rows = await cxn
        .select({ id: schema.media.id })
        .from(schema.media)
        .where(
            kind
                ? and(eq(schema.media.id, id), eq(schema.media.kind, kind))
                : eq(schema.media.id, id),
        )
        .limit(1);
    return rows.length > 0;
}
