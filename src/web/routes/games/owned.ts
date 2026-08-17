//! Owned game routes.
//!
//! Releases, consoles, and accessories are one table discriminated by kind:
//! they record the same hardware detail and the same ordered list of games. The
//! kind paths are sugar over the `kind` query parameter, while an item is
//! addressed by its identifier alone.

import { and, asc, desc, eq, inArray, like } from "drizzle-orm";
import type { SQLiteColumn } from "drizzle-orm/sqlite-core";
import { Elysia, t } from "elysia";

import {
    bool,
    BY_KIND,
    choice,
    define,
    ident,
    list,
    nullable,
    page,
    uuid,
    type Fields,
} from "../../../models";
import * as db from "../../../sql";
import * as schema from "../../../sql/schema";
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

/** What an owned item is. */
export const KINDS = ["release", "console", "extra"] as const;

export type Kind = (typeof KINDS)[number];

/** How a request body may name the games an item carries. */
export type Refs = { game?: string | undefined; games?: string[] | undefined };

/**
 * Resolves the games an item carries.
 *
 * A list is taken as given, while a lone identifier stands for a list of one.
 * Naming neither leaves the references alone.
 */
export function refs(given: Refs) {
    if (given.games !== undefined) return given.games;
    if (given.game !== undefined) return [given.game];
    return undefined;
}

/** Columns holding collection state, which default to false. */
const FLAGS = ["complete", "modified"];

/** Referenced game, as embedded in a resolved item. */
const Game = BY_KIND.get("game")!.item;

/** Column declarations, in wire order. */
const FIELDS: Fields = {
    id: uuid("Unique identifier."),
    kind: t.Unsafe<never>({
        description: "Item kind.",
        enum: [...KINDS],
    }),
    title: nullable(t.String(), "Title."),
    platform: nullable(t.String(), "Platform."),
    region: nullable(t.String(), "Region code."),
    model: nullable(t.String(), "Model name."),
    revision: nullable(t.String(), "Hardware revision."),
    serial: nullable(t.String(), "Serial number."),
    variant: nullable(t.String(), "Variant."),
    complete: bool("Complete-in-box status."),
    modified: bool("Hardware modification status."),
};

/** Columns a request body may set. The kind comes from the path. */
const COLUMNS = Object.keys(FIELDS).filter(
    (name) => name !== "id" && name !== "kind",
);

const { kind: discriminant, ...columns } = FIELDS;

/** Stored item, holding the columns of one owned thing. */
const Item = define(
    "OwnedGame",
    t.Object(columns, { description: "Owned game item." }),
);

/** Owned item, with its game references resolved. */
const Owned = t.Object(
    {
        kind: discriminant!,
        item: Item,
        games: list(Game, "Included games."),
    },
    { description: "Owned game record." },
);

const body: Fields = {
    title: t.Optional(FIELDS.title!),
    game: t.Optional(uuid("Included game; a single-element `games`.")),
    games: t.Optional(list(uuid("Game reference."), "Included games.")),
};
for (const name of COLUMNS) {
    if (name === "title") continue;
    body[name] = FLAGS.includes(name)
        ? t.Optional(nullable(t.Boolean(), FIELDS[name]!.description ?? ""))
        : t.Optional(FIELDS[name]!);
}

const Body = t.Object(body, { description: "Request body." });
const Patch = t.Partial(t.Object(body), {
    description: "Partial request body.",
});

const TAG = "games/owned";

const SORT = ["title", "platform", "model"];

export function router(cxn: db.Cxn) {
    const params = t.Object({ id: ident });

    /** Answers a request for an item that is not on file. */
    const missing = () => fail("not_found", "No owned game item with that ID.");
    const filters = {
        q: t.Optional(
            t.String({
                description: "Search title (case-insensitive substring).",
            }),
        ),
        game: t.Optional(
            t.String({
                format: "uuid",
                description: "Filter by included game ID.",
            }),
        ),
        platform: t.Optional(t.String({ description: "Filter by platform." })),
        sort: choice(SORT, "Field to sort by."),
        ...page,
    };
    const query = t.Object({
        kind: choice([...KINDS], "Filter by item kind."),
        ...filters,
    });

    const ref = schema.games_owned_ref;

    /** Loads the games referenced by each item, keyed by item ID. */
    const games = async (ids: string[]) => {
        const rows = await cxn
            .select({ owned: ref.owned, game: schema.games })
            .from(ref)
            .innerJoin(schema.games, eq(schema.games.id, ref.game))
            .where(inArray(ref.owned, ids))
            .orderBy(ref.owned, ref.idx);

        const out = new Map<string, unknown[]>();
        for (const row of rows) {
            const held = out.get(row.owned);
            if (held) held.push(row.game);
            else out.set(row.owned, [row.game]);
        }
        return out;
    };

    /** Replaces the games referenced by an item, preserving list order. */
    type Tx = Parameters<Parameters<db.Cxn["transaction"]>[0]>[0];
    const relate = (tx: Tx, owned: string, held: string[]) => {
        tx.delete(ref).where(eq(ref.owned, owned)).run();
        held.forEach((game, idx) => {
            tx.insert(ref)
                .values({ owned, game, idx })
                .onConflictDoNothing()
                .run();
        });
    };

    /** Wraps each row in the record envelope, resolving its game references. */
    const resolve = async (rows: { id: string }[]) => {
        const held = await games(rows.map((row) => row.id));
        return rows.map((row) => {
            const { kind, ...item } = row as Record<string, unknown>;
            return { kind, item, games: held.get(row.id) ?? [] };
        });
    };

    const fetch = async (id: string) => {
        const rows = await cxn
            .select()
            .from(schema.games_owned)
            .where(eq(schema.games_owned.id, id));
        return rows.length
            ? (await resolve(rows as { id: string }[]))[0]
            : undefined;
    };

    /** Splits a request body into column values and game references. */
    const split = (given: Record<string, unknown>, fill: boolean) => {
        const values: Record<string, unknown> = {};
        for (const name of COLUMNS) {
            const blank = FLAGS.includes(name) ? false : null;
            if (name in given) values[name] = given[name] ?? blank;
            else if (fill) values[name] = blank;
        }
        return { values, held: refs(given as Refs) };
    };

    type Query = {
        q?: string | undefined;
        game?: string | undefined;
        platform?: string | undefined;
        sort?: string | undefined;
        order?: string | undefined;
        limit?: number | undefined;
        offset?: number | undefined;
    };

    /** Lists items, optionally constrained to one kind. */
    const search = async (only: Kind | undefined, args: Query) => {
        // Apply filters
        const where = [
            only ? eq(schema.games_owned.kind, only) : undefined,
            args.q ? like(schema.games_owned.title, `%${args.q}%`) : undefined,
            args.game
                ? inArray(
                      schema.games_owned.id,
                      cxn
                          .select({ owned: ref.owned })
                          .from(ref)
                          .where(eq(ref.game, args.game)),
                  )
                : undefined,
            args.platform
                ? eq(schema.games_owned.platform, args.platform)
                : undefined,
        ];

        // Sort and paginate
        const columns: Record<string, SQLiteColumn> = {
            title: schema.games_owned.title,
            platform: schema.games_owned.platform,
            model: schema.games_owned.model,
        };
        const column = columns[args.sort ?? "title"]!;
        const sorted = cxn
            .select()
            .from(schema.games_owned)
            .where(and(...where))
            .orderBy((args.order === "asc" ? asc : desc)(column));
        const rows = await (args.limit === undefined
            ? sorted
            : sorted.limit(args.limit).offset(args.offset ?? 0));

        return resolve(rows as { id: string }[]);
    };

    /** Records a new item of the given kind. */
    const insert = (kind: Kind, given: unknown) => {
        const id = crypto.randomUUID();
        const { values, held } = split(given as Record<string, unknown>, true);
        cxn.transaction((tx) => {
            tx.insert(schema.games_owned)
                .values({
                    id,
                    kind,
                    ...values,
                } as typeof schema.games_owned.$inferInsert)
                .run();
            relate(tx, id, held ?? []);
        });
        return created(id);
    };

    const found = json(Owned);
    const listed = json(t.Array(Owned));

    return new Elysia({ prefix: "/games/owned", name: "owned:games" })
        .get(
            "",
            ({ query: args }) => search(args.kind as Kind | undefined, args),
            {
                query,
                detail: operation({
                    tag: TAG,
                    id: "listGamesOwned",
                    about: "List owned game items.",
                    responses: { 200: listed },
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
                    id: "fetchGamesOwned",
                    about: "Fetch an owned game item by ID.",
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .put(
            "/:id",
            async ({ params: args, body: given }) => {
                const { values, held } = split(
                    given as Record<string, unknown>,
                    true,
                );
                const changed = cxn.transaction((tx) => {
                    const res = tx
                        .update(schema.games_owned)
                        .set(values)
                        .where(eq(schema.games_owned.id, args.id))
                        .run();
                    if (db.affected(res)) relate(tx, args.id, held ?? []);
                    return db.affected(res);
                });
                if (!changed) return missing();
                return (await fetch(args.id))!;
            },
            {
                params,
                body: Body,
                parse: "json",
                detail: operation({
                    tag: TAG,
                    id: "updateGamesOwned",
                    about: "Update an owned game item.",
                    write: true,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .patch(
            "/:id",
            async ({ params: args, body: given }) => {
                const { values, held } = split(
                    given as Record<string, unknown>,
                    false,
                );

                // Apply present fields
                if (Object.keys(values).length || held) {
                    cxn.transaction((tx) => {
                        if (Object.keys(values).length) {
                            tx.update(schema.games_owned)
                                .set(values)
                                .where(eq(schema.games_owned.id, args.id))
                                .run();
                        }
                        if (held) relate(tx, args.id, held);
                    });
                }

                return (await fetch(args.id)) ?? missing();
            },
            {
                params,
                body: Patch,
                parse: "json",
                detail: operation({
                    tag: TAG,
                    id: "modifyGamesOwned",
                    about: "Modify an owned game item.",
                    write: true,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .delete(
            "/:id",
            async ({ params: args }) => {
                const res = await cxn
                    .delete(schema.games_owned)
                    .where(eq(schema.games_owned.id, args.id));
                return db.affected(res) ? empty(NO_CONTENT) : missing();
            },
            {
                params,
                detail: operation({
                    tag: TAG,
                    id: "removeGamesOwned",
                    about: "Delete an owned game item.",
                    write: true,
                    responses: { 204: bare, 404: failed },
                }),
            },
        )
        .get(`/releases`, ({ query: args }) => search("release", args), {
            query: t.Object(filters),
            detail: operation({
                tag: TAG,
                id: "listReleases",
                about: "List owned releases.",
                responses: { 200: listed },
            }),
        })
        .post(`/releases`, ({ body: given }) => insert("release", given), {
            body: Body,
            parse: "json",
            detail: operation({
                tag: TAG,
                id: "createRelease",
                about: "Create a release.",
                write: true,
                responses: { 201: json(Id), 500: failed },
            }),
        })
        .get(`/consoles`, ({ query: args }) => search("console", args), {
            query: t.Object(filters),
            detail: operation({
                tag: TAG,
                id: "listConsoles",
                about: "List owned consoles.",
                responses: { 200: listed },
            }),
        })
        .post(`/consoles`, ({ body: given }) => insert("console", given), {
            body: Body,
            parse: "json",
            detail: operation({
                tag: TAG,
                id: "createConsole",
                about: "Create a console.",
                write: true,
                responses: { 201: json(Id), 500: failed },
            }),
        })
        .get(`/extras`, ({ query: args }) => search("extra", args), {
            query: t.Object(filters),
            detail: operation({
                tag: TAG,
                id: "listExtras",
                about: "List owned extras.",
                responses: { 200: listed },
            }),
        })
        .post(`/extras`, ({ body: given }) => insert("extra", given), {
            body: Body,
            parse: "json",
            detail: operation({
                tag: TAG,
                id: "createExtra",
                about: "Create an extra.",
                write: true,
                responses: { 201: json(Id), 500: failed },
            }),
        });
}
