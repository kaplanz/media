//! Routes over any media kind.

import { and, asc, desc, eq, exists, inArray, sql } from "drizzle-orm";
import type { SQLiteColumn } from "drizzle-orm/sqlite-core";
import { Elysia, t } from "elysia";

import {
    BY_KIND,
    choice,
    ident,
    ITEMS,
    type Kind,
    KINDS,
    Media,
    page,
} from "../../models";
import * as db from "../../sql";
import * as schema from "../../sql/schema";
import { extend, logs, type Record, tags } from "../record";
import {
    bare,
    empty,
    fail,
    failed,
    json,
    NO_CONTENT,
    operation,
} from "../reply";

/** A row of the media table. */
type Row = { id: string; kind: string; created: number; updated: number };

// These routes answer across every kind, so they return the union of the
// records, discriminated by `kind`.
const found = json(Media);
const listed = json(t.Array(Media));

const selection = {
    id: schema.media.id,
    kind: schema.media.kind,
    created: schema.media.created,
    updated: schema.media.updated,
};

/** Restricts a media query to items carrying a tag. */
const tagged = (cxn: db.Cxn, label: string) =>
    exists(
        cxn
            .select({ found: sql`1` })
            .from(schema.tags)
            .where(
                and(
                    eq(schema.tags.media, schema.media.id),
                    eq(schema.tags.label, label),
                ),
            ),
    );

/**
 * Loads the kind-specific item of each row and rebuilds the records in the
 * order the rows were returned.
 *
 * A row whose item is missing is dropped, since the record would have no body.
 */
async function collect(
    cxn: db.Cxn,
    rows: Row[],
): Promise<Record<{ id: string }>[]> {
    // Load items, partitioned by kind
    const items = new Map<string, { kind: Kind; item: { id: string } }>();
    await Promise.all(
        ITEMS.map(async (decl) => {
            const owned = rows
                .filter((row) => row.kind === decl.kind)
                .map((row) => row.id);
            if (!owned.length) return;
            const loaded = await cxn
                .select()
                .from(decl.table)
                .where(inArray(decl.table.id, owned));
            for (const item of loaded) {
                items.set((item as { id: string }).id, {
                    kind: decl.kind,
                    item: item as { id: string },
                });
            }
        }),
    );

    // Load metadata
    const ids = rows.map((row) => row.id);
    const [applied, activity] = await Promise.all([
        tags(cxn, ids),
        logs(cxn, ids),
    ]);

    const out = rows.flatMap((row) => {
        const found = items.get(row.id);
        if (!found) return [];
        return [
            {
                kind: found.kind,
                item: found.item,
                meta: { created: row.created, updated: row.updated },
                logs: activity.get(row.id) ?? [],
                tags: applied.get(row.id) ?? [],
            },
        ];
    });
    await extend(cxn, out);
    return out;
}

export function router(cxn: db.Cxn) {
    const params = t.Object({ id: ident });

    /** Answers a request for an item that is not on file. */
    const missing = () => fail("not_found", "No media with that ID.");
    const sort: globalThis.Record<string, SQLiteColumn> = {
        created: schema.media.created,
        updated: schema.media.updated,
    };

    return new Elysia({ name: "media" })
        .get(
            "/",
            async ({ query: args }) => {
                // Sort and paginate
                const column = sort[args.sort ?? "created"]!;
                const sorted = cxn
                    .select(selection)
                    .from(schema.media)
                    .where(args.tag ? tagged(cxn, args.tag) : undefined)
                    .orderBy((args.order === "asc" ? asc : desc)(column));
                const rows = await (args.limit === undefined
                    ? sorted
                    : sorted.limit(args.limit).offset(args.offset ?? 0));

                return collect(cxn, rows);
            },
            {
                query: t.Object({
                    tag: t.Optional(t.String({ description: "Filter by tag." })),
                    sort: choice(["created", "updated"], "Field to sort by."),
                    ...page,
                }),
                detail: operation({
                    tag: "media",
                    id: "listMedia",
                    about: "List all media.",
                    responses: { 200: listed },
                }),
            },
        )
        .get(
            "/:id",
            async ({ params: args }) => {
                const rows = await cxn
                    .select(selection)
                    .from(schema.media)
                    .where(eq(schema.media.id, args.id));
                const row = rows[0];
                if (!row || !BY_KIND.has(row.kind as Kind)) return missing();
                const records = await collect(cxn, [row]);
                return records[0] ?? missing();
            },
            {
                params,
                detail: operation({
                    tag: "media",
                    id: "fetchMedia",
                    about: "Fetch any media item by ID.",
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .delete(
            "/:id",
            async ({ params: args }) => {
                const res = await cxn
                    .delete(schema.media)
                    .where(eq(schema.media.id, args.id));
                return db.affected(res) ? empty(NO_CONTENT) : missing();
            },
            {
                params,
                detail: operation({
                    tag: "media",
                    id: "removeMedia",
                    about: "Delete any media item by ID.",
                    write: true,
                    responses: { 204: bare, 404: failed },
                }),
            },
        )
        .get(
            "/tags",
            async ({ query: args }) => {
                const labels = cxn.selectDistinct({ label: schema.tags.label });
                const rows = args.kind
                    ? await labels
                          .from(schema.tags)
                          .innerJoin(
                              schema.media,
                              eq(schema.tags.media, schema.media.id),
                          )
                          .where(eq(schema.media.kind, args.kind))
                          .orderBy(schema.tags.label)
                    : await labels.from(schema.tags).orderBy(schema.tags.label);
                return rows.map((row) => row.label);
            },
            {
                query: t.Object({
                    kind: choice([...KINDS], "Filter by media kind."),
                }),
                detail: operation({
                    tag: "media",
                    id: "listTags",
                    about: "List all distinct tags.",
                    responses: { 200: json(t.Array(t.String())) },
                }),
            },
        )
        .get(
            "/tags/:tag",
            async ({ params: args }) => {
                const rows = await cxn
                    .select(selection)
                    .from(schema.media)
                    .where(tagged(cxn, args.tag))
                    .orderBy(desc(schema.media.created));
                if (!rows.length) {
                    return fail("not_found", "No media carries that tag.");
                }
                return collect(cxn, rows);
            },
            {
                params: t.Object({ tag: t.String({ description: "Tag label." }) }),
                detail: operation({
                    tag: "media",
                    id: "fetchMediaByTag",
                    about: "Fetch media items by tag.",
                    responses: { 200: listed, 404: failed },
                }),
            },
        );
}
