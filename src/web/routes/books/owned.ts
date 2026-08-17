//! Owned book routes.
//!
//! A book that is owned is a separate fact from a book that has been read, so
//! copies are tracked by ISBN and carry no reference to a reading entry. The
//! matching entry is resolved opportunistically, since `books.isbn` is unique.

import { and, asc, desc, eq, inArray, like } from "drizzle-orm";
import type { SQLiteColumn } from "drizzle-orm/sqlite-core";
import { Elysia, t } from "elysia";

import {
    BY_KIND,
    choice,
    define,
    ident,
    nullable,
    page,
    str,
    uuid,
    type Fields,
} from "../../../models";
import * as db from "../../../sql";
import * as schema from "../../../sql/schema";
import { extend } from "../../record";
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
} from "../../reply";

/** Reading entry, as embedded in a resolved copy. */
const Book = BY_KIND.get("book")!.item;

/** Column declarations, in wire order. */
const FIELDS: Fields = {
    id: uuid("Unique identifier."),
    isbn: str("ISBN-13."),
    title: nullable(t.String(), "Title."),
    edition: nullable(t.String(), "Edition."),
};

const COLUMNS = Object.keys(FIELDS).filter((name) => name !== "id");

/** Stored copy, holding the columns of one owned book. */
const Item = define(
    "OwnedBook",
    t.Object(FIELDS, { description: "Owned book copy." }),
);

/** Owned copy, with its reading entry resolved. */
const Owned = t.Object(
    {
        item: Item,
        book: t.Union([Book, t.Null()], {
            description: "Matching reading item, if one is recorded.",
        }),
    },
    { description: "Owned book record." },
);

const body: Fields = { isbn: FIELDS.isbn! };
for (const name of COLUMNS) {
    if (name === "isbn") continue;
    body[name] = t.Optional(FIELDS[name]!);
}

const Body = t.Object(body, { description: "Request body." });
const Patch = t.Partial(t.Object(body), {
    description: "Partial request body.",
});

const TAG = "books/owned";

export function router(cxn: db.Cxn) {
    const params = t.Object({ id: ident });

    /** Answers a request for a copy that is not on file. */
    const missing = () => fail("not_found", "No owned book with that ID.");
    const query = t.Object({
        q: t.Optional(
            t.String({ description: "Search title (case-insensitive substring)." }),
        ),
        isbn: t.Optional(t.String({ description: "Filter by ISBN-13." })),
        sort: choice(["isbn", "title", "edition"], "Field to sort by."),
        ...page,
    });

    /** Attaches the reading entry that shares each copy's ISBN. */
    const resolve = async (
        rows: (typeof schema.books_owned.$inferSelect)[],
    ) => {
        if (!rows.length) return [];
        const books = await cxn
            .select()
            .from(schema.books)
            .where(
                inArray(
                    schema.books.isbn,
                    rows.map((row) => row.isbn),
                ),
            );
        await extend(
            cxn,
            books.map((item) => ({ kind: "book" as const, item })),
        );
        const found = new Map(books.map((book) => [book.isbn, book]));
        return rows.map((row) => ({
            item: row,
            book: found.get(row.isbn) ?? null,
        }));
    };

    const fetch = async (id: string) => {
        const rows = await cxn
            .select()
            .from(schema.books_owned)
            .where(eq(schema.books_owned.id, id));
        return rows.length ? (await resolve(rows))[0] : undefined;
    };

    /** Fills every column, since an absent field clears its value. */
    const values = (given: Record<string, unknown>) =>
        Object.fromEntries(COLUMNS.map((name) => [name, given[name] ?? null]));

    const found = json(Owned);

    return new Elysia({ prefix: "/books/owned", name: "owned:books" })
        .get(
            "",
            async ({ query: args }) => {
                // Apply filters
                const where = [
                    args.q
                        ? like(schema.books_owned.title, `%${args.q}%`)
                        : undefined,
                    args.isbn
                        ? eq(schema.books_owned.isbn, args.isbn)
                        : undefined,
                ];

                // Sort and paginate
                const columns: Record<string, SQLiteColumn> = {
                    isbn: schema.books_owned.isbn,
                    title: schema.books_owned.title,
                    edition: schema.books_owned.edition,
                };
                const column = columns[args.sort ?? "title"]!;
                const sorted = cxn
                    .select()
                    .from(schema.books_owned)
                    .where(and(...where))
                    .orderBy((args.order === "asc" ? asc : desc)(column));
                const rows = await (args.limit === undefined
                    ? sorted
                    : sorted.limit(args.limit).offset(args.offset ?? 0));

                return resolve(rows);
            },
            {
                query,
                detail: operation({
                    tag: TAG,
                    id: "listBooksOwned",
                    about: "List owned books.",
                    responses: { 200: json(t.Array(Owned)) },
                }),
            },
        )
        .post(
            "",
            async ({ body: given }) => {
                const id = crypto.randomUUID();
                await cxn.insert(schema.books_owned).values({
                    id,
                    ...values(given as Record<string, unknown>),
                } as typeof schema.books_owned.$inferInsert);
                return created(id);
            },
            {
                body: Body,
                parse: "json",
                detail: operation({
                    tag: TAG,
                    id: "createBooksOwned",
                    about: "Record an owned book.",
                    write: true,
                    responses: { 201: json(Id), 500: failed },
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
                    tag: TAG,
                    id: "fetchBooksOwned",
                    about: "Fetch an owned book by ID.",
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .put(
            "/:id",
            async ({ params: args, body: given }) => {
                const res = await cxn
                    .update(schema.books_owned)
                    .set(values(given as Record<string, unknown>))
                    .where(eq(schema.books_owned.id, args.id));
                if (!db.affected(res)) return missing();
                return (await fetch(args.id))!;
            },
            {
                params,
                body: Body,
                parse: "json",
                detail: operation({
                    tag: TAG,
                    id: "updateBooksOwned",
                    about: "Update an owned book.",
                    write: true,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .patch(
            "/:id",
            async ({ params: args, body: given }) => {
                // Apply present fields
                const fields = given as Record<string, unknown>;
                if (Object.keys(fields).length) {
                    const res = await cxn
                        .update(schema.books_owned)
                        .set(fields)
                        .where(eq(schema.books_owned.id, args.id));
                    if (!db.affected(res)) return missing();
                }

                return (await fetch(args.id)) ?? missing();
            },
            {
                params,
                body: Patch,
                parse: "json",
                detail: operation({
                    tag: TAG,
                    id: "modifyBooksOwned",
                    about: "Modify an owned book.",
                    write: true,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .delete(
            "/:id",
            async ({ params: args }) => {
                const res = await cxn
                    .delete(schema.books_owned)
                    .where(eq(schema.books_owned.id, args.id));
                return db.affected(res) ? empty(NO_CONTENT) : missing();
            },
            {
                params,
                detail: operation({
                    tag: TAG,
                    id: "removeBooksOwned",
                    about: "Delete an owned book.",
                    write: true,
                    responses: { 204: bare, 404: failed },
                }),
            },
        );
}
