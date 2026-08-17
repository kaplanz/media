//! Per-kind media routes.
//!
//! One router serves books, films, games, links, and shows. The kind's
//! declaration supplies the table, the searchable column, and the sortable
//! columns; every route is otherwise identical across kinds.

import { and, asc, desc, eq, exists, like, sql } from "drizzle-orm";
import { Elysia, t } from "elysia";

import {
    choice,
    ident,
    type Item,
    narrowed,
    page,
    pascal,
} from "../../models";
import * as db from "../../sql";
import * as schema from "../../sql/schema";
import { wrap } from "../record";
import {
    bare,
    created,
    empty,
    fail,
    failed,
    Id,
    json,
    NO_CONTENT,
    operation,
} from "../reply";

export function router(cxn: db.Cxn, decl: Item) {
    const { kind, tag, one, many, table, search, sort } = decl;
    const Record = narrowed(kind);

    // Operation IDs are global, so each is qualified by what it acts on
    const It = pascal(kind);
    const Them = pascal(tag);

    /** Answers a request for an item that is not on file. */
    const missing = () => fail("not_found", `No ${kind} with that ID.`);
    const params = t.Object({ id: ident });

    const query = t.Object({
        q: t.Optional(
            t.String({ description: "Search title (case-insensitive substring)." }),
        ),
        tag: t.Optional(t.String({ description: "Filter by tag." })),
        sort: choice(Object.keys(sort), "Field to sort by."),
        ...page,
    });

    /** Selects a kind's columns alongside its shared metadata. */
    const join = () =>
        cxn
            .select({
                item: table,
                created: schema.media.created,
                updated: schema.media.updated,
            })
            .from(table)
            .innerJoin(schema.media, eq(table.id, schema.media.id));

    type Rows = Awaited<ReturnType<ReturnType<typeof join>["execute"]>>;

    const envelope = (rows: Rows) =>
        wrap(
            cxn,
            kind,
            rows as { item: { id: string }; created: number; updated: number }[],
        );

    const fetch = async (id: string) => {
        const rows = await join().where(eq(table.id, id));
        return rows.length ? (await envelope(rows))[0] : undefined;
    };

    const found = json(Record);

    return new Elysia({ prefix: `/${tag}`, name: `item:${tag}` })
        .get(
            "",
            async ({ query: args }) => {
                // Apply filters
                const filters = [
                    args.q ? like(search, `%${args.q}%`) : undefined,
                    args.tag
                        ? exists(
                              cxn
                                  .select({ found: sql`1` })
                                  .from(schema.tags)
                                  .where(
                                      and(
                                          eq(schema.tags.media, table.id),
                                          eq(schema.tags.label, args.tag),
                                      ),
                                  ),
                          )
                        : undefined,
                ];

                // Sort and paginate
                const column = sort[args.sort ?? "created"]!;
                const sorted = join()
                    .where(and(...filters))
                    .orderBy((args.order === "asc" ? asc : desc)(column));
                const rows = await (args.limit === undefined
                    ? sorted
                    : sorted.limit(args.limit).offset(args.offset ?? 0));

                return envelope(rows);
            },
            {
                query,
                detail: operation({
                    tag,
                    id: `list${Them}`,
                    about: `List ${many}.`,
                    responses: { 200: json(t.Array(Record)) },
                }),
            },
        )
        .get(
            "/:id",
            async ({ params: args }) =>
                (await fetch(args.id)) ?? missing(),
            {
                params,
                detail: operation({
                    tag,
                    id: `fetch${It}`,
                    about: `Fetch ${one} by ID.`,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .post(
            "",
            ({ body }) => {
                const id = crypto.randomUUID();
                const at = db.timestamp();

                // NOTE: No insert trigger, so both rows are written here
                cxn.transaction((tx) => {
                    tx.insert(schema.media)
                        .values({ id, kind, created: at, updated: at })
                        .run();
                    tx.insert(table)
                        .values({ id, ...(body as object) })
                        .run();
                });

                return created(id);
            },
            {
                body: decl.body,
                parse: "json",
                detail: operation({
                    tag,
                    id: `create${It}`,
                    about: `Create ${one}.`,
                    write: true,
                    responses: { 201: json(Id), 500: failed },
                }),
            },
        )
        .put(
            "/:id",
            async ({ params: args, body }) => {
                // Absent fields clear their column
                const given = body as Record<string, unknown>;
                const values = Object.fromEntries(
                    decl.columns.map((name) => [name, given[name] ?? null]),
                );

                const res = await cxn
                    .update(table)
                    .set(values)
                    .where(eq(table.id, args.id));
                if (!db.affected(res)) return missing();
                await db.touch(cxn, args.id);

                return (await fetch(args.id))!;
            },
            {
                params,
                body: decl.body,
                parse: "json",
                detail: operation({
                    tag,
                    id: `update${It}`,
                    about: `Update ${one}.`,
                    write: true,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .patch(
            "/:id",
            async ({ params: args, body }) => {
                // Apply present fields
                const values = body as Record<string, unknown>;
                if (Object.keys(values).length) {
                    const res = await cxn
                        .update(table)
                        .set(values)
                        .where(eq(table.id, args.id));
                    if (!db.affected(res)) return missing();
                    await db.touch(cxn, args.id);
                }

                return (await fetch(args.id)) ?? missing();
            },
            {
                params,
                body: decl.patch,
                parse: "json",
                detail: operation({
                    tag,
                    id: `modify${It}`,
                    about: `Modify ${one}.`,
                    write: true,
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
                    tag,
                    id: `remove${It}`,
                    about: `Delete ${one}.`,
                    write: true,
                    responses: { 204: bare, 404: failed },
                }),
            },
        );
}
