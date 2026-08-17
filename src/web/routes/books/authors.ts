//! Book author routes.
//!
//! Authors mirror tags, except that their order is meaningful: the list is
//! stored with a sequence number, so replacing it fixes the order and adding
//! one appends to the end.

import { and, eq, max } from "drizzle-orm";
import { Elysia, t } from "elysia";

import { ident } from "../../../models";
import * as db from "../../../sql";
import * as schema from "../../../sql/schema";
import { exists } from "../../record";
import { bare, fail, failed, json, operation } from "../../reply";

import { ops } from "../tags";

const TAG = "books";

export function router(cxn: db.Cxn) {
    const id = ops("Book", "Authors", "Author");
    const names = json(t.Array(t.String()));
    const params = t.Object({ id: ident });
    const target = t.Object({
        id: ident,
        name: t.String({ description: "Author name." }),
    });

    /** Rejects a request for a book that does not exist. */
    const guard = async (book: string) =>
        (await exists(cxn, book, "book"))
            ? undefined
            : fail("not_found", "No book with that ID.");

    /** Loads the authors of one book, in listed order. */
    const load = async (book: string) => {
        const rows = await cxn
            .select({ name: schema.books_author.name })
            .from(schema.books_author)
            .where(eq(schema.books_author.book, book))
            .orderBy(schema.books_author.idx);
        return rows.map((row) => row.name);
    };

    /** Returns the sequence number an appended author takes. */
    const next = async (book: string) => {
        const rows = await cxn
            .select({ idx: max(schema.books_author.idx) })
            .from(schema.books_author)
            .where(eq(schema.books_author.book, book));
        return (rows[0]?.idx ?? -1) + 1;
    };

    return new Elysia({ prefix: `/${TAG}`, name: "authors" })
        .get(
            "/:id/authors",
            async ({ params: args }) =>
                (await guard(args.id)) ?? load(args.id),
            {
                params,
                detail: operation({
                    tag: TAG,
                    id: id.list,
                    about: "List authors for a book.",
                    responses: { 200: names, 404: failed },
                }),
            },
        )
        .put(
            "/:id/authors",
            async ({ params: args, body }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                cxn.transaction((tx) => {
                    tx.delete(schema.books_author)
                        .where(eq(schema.books_author.book, args.id))
                        .run();
                    body.forEach((name, idx) => {
                        tx.insert(schema.books_author)
                            .values({ book: args.id, name, idx })
                            .onConflictDoNothing()
                            .run();
                    });
                });
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params,
                body: t.Array(t.String()),
                parse: "json",
                detail: operation({
                    tag: TAG,
                    id: id.set,
                    about: "Replace authors for a book.",
                    write: true,
                    responses: { 200: names, 404: failed },
                }),
            },
        )
        .put(
            "/:id/authors/:name",
            async ({ params: args }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                await cxn
                    .insert(schema.books_author)
                    .values({
                        book: args.id,
                        name: args.name,
                        idx: await next(args.id),
                    })
                    .onConflictDoNothing();
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params: target,
                detail: operation({
                    tag: TAG,
                    id: id.insert,
                    about: "Add an author to a book.",
                    write: true,
                    responses: { 200: names, 404: failed },
                }),
            },
        )
        .delete(
            "/:id/authors/:name",
            async ({ params: args }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                const res = await cxn
                    .delete(schema.books_author)
                    .where(
                        and(
                            eq(schema.books_author.book, args.id),
                            eq(schema.books_author.name, args.name),
                        ),
                    );
                if (!db.affected(res)) {
                    return fail("not_found", "No such author on this book.");
                }
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params: target,
                detail: operation({
                    tag: TAG,
                    id: id.remove,
                    about: "Remove an author from a book.",
                    write: true,
                    responses: { 200: names, 404: failed },
                }),
            },
        );
}
