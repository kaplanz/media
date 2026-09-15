//! Owned game ROM routes.
//!
//! A dump is a fact about the cartridge or disc held, not about the game, so
//! it hangs off the owned item. The bytes live on the filesystem under
//! `{root}/{owned}/{rom}`, where both segments are immutable keys, so no edit
//! ever moves a file.
//!
//! Digests are recorded when the bytes arrive rather than computed when they
//! are asked for: listing then costs an index read instead of rehashing every
//! file, and asking which item holds a known file becomes a lookup.
//!
//! Each path says one thing. The nested paths are about ownership, so they
//! list what an item holds and accept what is dumped from it. A dump is keyed
//! by its own identifier, so the flat paths address one wherever it belongs.

import { mkdir, rename, rm } from "node:fs/promises";
import { join } from "node:path";
import { crc32 } from "node:zlib";

import { and, asc, count, desc, eq, inArray, like } from "drizzle-orm";
import type { SQLiteColumn } from "drizzle-orm/sqlite-core";
import { Elysia, t } from "elysia";

import {
    bool,
    choice,
    define,
    ident,
    int,
    nullable,
    page,
    str,
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

/** Digests recorded for every dump, in wire order. */
const DIGESTS = ["crc32", "md5", "sha1", "sha256"] as const;

/** Content digests, as one dump reports them. */
type Digest = Record<(typeof DIGESTS)[number], string>;

/** What a dump measures out to. */
type Measure = { size: number; hash: Digest };

/** Column declarations, in wire order. */
const FIELDS: Fields = {
    id: uuid("Unique identifier."),
    owned: uuid("Owned item this was dumped from."),
    name: nullable(t.String(), "Filename, as recorded when dumped."),
    size: int("Size in bytes."),
    dumped: int("Dump date (Unix seconds)."),
};

/** Content digests, as lowercase hexadecimal. */
const Hash = define(
    "Hash",
    t.Object(
        {
            crc32: str("CRC-32."),
            md5: str("MD5."),
            sha1: str("SHA-1."),
            sha256: str("SHA-256."),
        },
        { description: "Content digests, as lowercase hexadecimal." },
    ),
);

/**
 * One dump, as served.
 *
 * `owned` is carried even where the path already names it, so one shape
 * describes a dump wherever it appears: on its own, in a list of everything,
 * or beside the item it came from.
 *
 * `present` is answered from the filesystem rather than the database, since
 * the two may disagree: a collection loaded from a dump carries every row and
 * none of the bytes.
 */
export const Rom = define(
    "Rom",
    t.Object(
        {
            id: FIELDS.id!,
            owned: FIELDS.owned!,
            name: FIELDS.name!,
            size: FIELDS.size!,
            hash: Hash,
            dumped: FIELDS.dumped!,
            present: bool("Whether the file is held in the store."),
        },
        { description: "Dumped ROM." },
    ),
);

/** What rehashing a dump found. */
const Verify = t.Object(
    {
        intact: bool("Whether the file still matches what was recorded."),
        size: int("Size in bytes, as measured."),
        hash: Hash,
    },
    { description: "Verification result." },
);

const Patch = t.Object(
    {
        name: t.Optional(nullable(t.String(), "Filename.")),
        idx: t.Optional(t.Integer({ description: "Position in the list." })),
    },
    { description: "Partial request body." },
);

const TAG = "games/owned";

const SORT = ["name", "size", "dumped"];

/** Returns the directory holding one owned item's dumps. */
export const holding = (root: string, owned: string) => join(root, owned);

/** Returns the path to one dump. */
const located = (root: string, row: { id: string; owned: string }) =>
    join(root, row.owned, row.id);

/**
 * Wraps a row for the wire, nesting its digests and checking the store.
 *
 * Shared with the owned routes, which carry a record's dumps alongside it.
 */
export const served = async (
    root: string,
    row: typeof schema.games_owned_rom.$inferSelect,
) => {
    const { crc32: crc, md5, sha1, sha256, idx: _, ...rest } = row;
    return {
        ...rest,
        hash: { crc32: crc, md5, sha1, sha256 },
        present: await Bun.file(located(root, row)).exists(),
    };
};

/**
 * Loads the dumps of each owned item, keyed by item identifier.
 *
 * Every item is answered by one query, so listing a collection costs the same
 * number of round trips whether it holds one dump or a thousand.
 */
export async function dumps(cxn: db.Cxn, root: string, ids: string[]) {
    const out = new Map<string, unknown[]>();
    if (!ids.length) return out;

    const rom = schema.games_owned_rom;
    const rows = await cxn
        .select()
        .from(rom)
        .where(inArray(rom.owned, ids))
        .orderBy(rom.owned, rom.idx);

    for (const row of rows) {
        const one = await served(root, row);
        const held = out.get(row.owned);
        if (held) held.push(one);
        else out.set(row.owned, [one]);
    }
    return out;
}

/**
 * Names the downloaded file.
 *
 * RFC 6266's extended form percent-encodes the whole name, which carries a
 * non-ASCII name correctly and leaves nothing able to close the header early.
 * The characters `'()!*` are left alone by `encodeURIComponent` but are not
 * `attr-char`, so they are encoded here: a ROM filename contains parentheses
 * more often than not.
 */
const disposition = (name: string) =>
    `attachment; filename*=UTF-8''${encodeURIComponent(name).replace(
        /['()!*]/g,
        (ch) => `%${ch.charCodeAt(0).toString(16).toUpperCase()}`,
    )}`;

/**
 * Digests a stream, writing it out when given somewhere to put it.
 *
 * The bytes are never held whole, so a multi-gigabyte image costs one chunk of
 * memory rather than its own size.
 */
async function absorb(
    body: ReadableStream<Uint8Array>,
    to?: string,
): Promise<Measure> {
    const md5 = new Bun.CryptoHasher("md5");
    const sha1 = new Bun.CryptoHasher("sha1");
    const sha256 = new Bun.CryptoHasher("sha256");
    let crc = 0;
    let size = 0;

    const sink = to ? Bun.file(to).writer() : undefined;
    const reader = body.getReader();
    for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        md5.update(value);
        sha1.update(value);
        sha256.update(value);
        crc = crc32(value, crc);
        size += value.byteLength;
        if (sink) await sink.write(value);
    }
    if (sink) await sink.end();

    return {
        size,
        hash: {
            crc32: (crc >>> 0).toString(16).padStart(8, "0"),
            md5: md5.digest("hex"),
            sha1: sha1.digest("hex"),
            sha256: sha256.digest("hex"),
        },
    };
}

export function router(cxn: db.Cxn, root: string) {
    const params = t.Object({ id: ident });
    const target = t.Object({ rom: ident });

    /** Answers a request naming an owned item that is not on file. */
    const absent = () => fail("not_found", "No owned game item with that ID.");
    /** Answers a request for a dump that is not on file. */
    const missing = () => fail("not_found", "No ROM with that ID.");
    /** Answers a request for bytes the store does not hold. */
    const gone = () => fail("not_found", "That ROM is not held in the store.");

    const rom = schema.games_owned_rom;

    /** Reports whether an owned item is on file. */
    const owns = async (id: string) =>
        (
            await cxn
                .select({ id: schema.games_owned.id })
                .from(schema.games_owned)
                .where(eq(schema.games_owned.id, id))
        ).length > 0;

    /** Returns the position a new dump takes, which is last. */
    const following = async (owned: string) =>
        (
            await cxn
                .select({ held: count() })
                .from(rom)
                .where(eq(rom.owned, owned))
        )[0]!.held;

    /** Loads one dump by its own identifier. */
    const find = async (id: string) =>
        (await cxn.select().from(rom).where(eq(rom.id, id)))[0];

    const found = json(Rom);

    return new Elysia({ prefix: "/games/owned", name: "roms:games" })
        .get(
            "/roms",
            async ({ query: args }) => {
                // Apply filters
                const where = [
                    args.q ? like(rom.name, `%${args.q}%`) : undefined,
                    args.owned ? eq(rom.owned, args.owned) : undefined,
                    args.sha1 ? eq(rom.sha1, args.sha1) : undefined,
                ];

                // Sort and paginate
                const columns: Record<string, SQLiteColumn> = {
                    name: rom.name,
                    size: rom.size,
                    dumped: rom.dumped,
                };
                const column = columns[args.sort ?? "dumped"]!;
                const sorted = cxn
                    .select()
                    .from(rom)
                    .where(and(...where))
                    .orderBy((args.order === "asc" ? asc : desc)(column));
                const rows = await (args.limit === undefined
                    ? sorted
                    : sorted.limit(args.limit).offset(args.offset ?? 0));

                return Promise.all(rows.map((row) => served(root, row)));
            },
            {
                query: t.Object({
                    q: t.Optional(
                        t.String({
                            description:
                                "Search filename (case-insensitive substring).",
                        }),
                    ),
                    owned: t.Optional(
                        t.String({
                            format: "uuid",
                            description: "Filter by owned item ID.",
                        }),
                    ),
                    sha1: t.Optional(
                        t.String({ description: "Filter by SHA-1." }),
                    ),
                    sort: choice(SORT, "Field to sort by."),
                    ...page,
                }),
                ...operation({
                    tag: TAG,
                    id: "listRoms",
                    about: "List every dumped ROM.",
                    responses: { 200: json(t.Array(Rom)) },
                }),
            },
        )
        .get(
            "/roms/:rom",
            async ({ params: args }) => {
                const row = await find(args.rom);
                return row ? served(root, row) : missing();
            },
            {
                params: target,
                ...operation({
                    tag: TAG,
                    id: "fetchRom",
                    about: "Fetch a ROM by ID.",
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .patch(
            "/roms/:rom",
            async ({ params: args, body: given }) => {
                const fields = given as Record<string, unknown>;
                if (Object.keys(fields).length) {
                    await cxn
                        .update(rom)
                        .set(fields)
                        .where(eq(rom.id, args.rom));
                }
                const row = await find(args.rom);
                return row ? served(root, row) : missing();
            },
            {
                params: target,
                body: Patch,
                parse: "json",
                ...operation({
                    tag: TAG,
                    id: "modifyRom",
                    about: "Modify a ROM's name or position.",
                    auth: true,
                    responses: { 200: found, 404: failed },
                }),
            },
        )
        .delete(
            "/roms/:rom",
            async ({ params: args }) => {
                const row = await find(args.rom);
                if (!row) return missing();
                await cxn.delete(rom).where(eq(rom.id, args.rom));
                await rm(located(root, row), { force: true });
                return empty(NO_CONTENT);
            },
            {
                params: target,
                ...operation({
                    tag: TAG,
                    id: "removeRom",
                    about: "Delete a ROM.",
                    auth: true,
                    responses: { 204: bare, 404: failed },
                }),
            },
        )
        .get(
            "/roms/:rom/data",
            async ({ params: args, request }) => {
                const row = await find(args.rom);
                if (!row) return missing();

                // The recorded digest names the content, so a revalidation is
                // answered without opening the file
                const etag = `"${row.sha1}"`;
                if (request.headers.get("if-none-match") === etag) {
                    const headers = { etag };
                    return new Response(null, { status: 304, headers });
                }

                const file = Bun.file(located(root, row));
                if (!(await file.exists())) return gone();
                return new Response(file, {
                    headers: {
                        "content-type": "application/octet-stream",
                        "content-length": String(row.size),
                        "content-disposition": disposition(row.name ?? row.id),
                        etag,
                    },
                });
            },
            {
                params: target,
                ...operation({
                    tag: TAG,
                    id: "fetchRomData",
                    about: "Download a ROM's bytes.",
                    auth: true,
                    responses: { 200: bare, 304: bare, 404: failed },
                }),
            },
        )
        .get(
            "/roms/:rom/verify",
            async ({ params: args }) => {
                const row = await find(args.rom);
                if (!row) return missing();

                const file = Bun.file(located(root, row));
                if (!(await file.exists())) return gone();

                const measure = await absorb(file.stream());
                const intact =
                    measure.size === row.size &&
                    DIGESTS.every((kind) => measure.hash[kind] === row[kind]);
                return { intact, size: measure.size, hash: measure.hash };
            },
            {
                params: target,
                ...operation({
                    tag: TAG,
                    id: "verifyRom",
                    about: "Rehash a ROM and compare it to what was recorded.",
                    auth: true,
                    responses: { 200: json(Verify), 404: failed },
                }),
            },
        )
        .get(
            "/:id/roms",
            async ({ params: args }) => {
                if (!(await owns(args.id))) return absent();
                const rows = await cxn
                    .select()
                    .from(rom)
                    .where(eq(rom.owned, args.id))
                    .orderBy(asc(rom.idx));
                return Promise.all(rows.map((row) => served(root, row)));
            },
            {
                params,
                ...operation({
                    tag: TAG,
                    id: "listGamesOwnedRoms",
                    about: "List the ROMs dumped from an owned game item.",
                    responses: { 200: json(t.Array(Rom)), 404: failed },
                }),
            },
        )
        .post(
            "/:id/roms",
            async ({ params: args, query, request }) => {
                if (!(await owns(args.id))) return absent();
                if (!request.body) {
                    return fail("invalid_body", "Request body is empty.");
                }

                // Write beside its siblings, then move into place, so a failed
                // upload leaves neither a half-file nor a row without bytes
                const id = crypto.randomUUID();
                const to = located(root, { id, owned: args.id });
                const temp = `${to}.part`;
                await mkdir(holding(root, args.id), { recursive: true });
                let measure: Measure;
                try {
                    measure = await absorb(request.body, temp);
                } catch (cause) {
                    // A client hanging up mid-stream leaves a partial file
                    // that nothing would ever name again
                    await rm(temp, { force: true });
                    throw cause;
                }

                // Refuse a transfer that did not arrive whole
                const stated = request.headers.get("content-length");
                if (stated !== null && Number(stated) !== measure.size) {
                    await rm(temp, { force: true });
                    return fail(
                        "invalid_body",
                        "Request body is shorter than its Content-Length.",
                    );
                }

                await rename(temp, to);
                await cxn.insert(rom).values({
                    id,
                    owned: args.id,
                    name: query.name ?? null,
                    size: measure.size,
                    ...measure.hash,
                    dumped: query.dumped ?? db.timestamp(),
                    idx: await following(args.id),
                });
                return created(id);
            },
            {
                params,
                query: t.Object({
                    name: t.Optional(
                        t.String({
                            description: "Filename, recorded as given.",
                        }),
                    ),
                    dumped: t.Optional(
                        t.Integer({
                            description:
                                "Dump date (Unix seconds); defaults to now.",
                        }),
                    ),
                }),
                ...operation({
                    tag: TAG,
                    id: "createRom",
                    about: "Upload a ROM dumped from an owned game item.",
                    auth: true,
                    responses: { 201: json(Id), 404: failed, 422: failed },
                }),
            },
        );
}
