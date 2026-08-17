//! Dump subcommand.

import { asc, type SQL, sql } from "drizzle-orm";

import { ITEMS, type Kind } from "../models";
import * as db from "../sql";
import * as schema from "../sql/schema";
import { extend, logs, tags } from "../web/record";

/** Serialization format. */
export const FORMATS = ["json", "sql"] as const;

export type Format = (typeof FORMATS)[number];

/** Dump arguments. */
export type Args = {
    /** SQLite database file. */
    db: string;
    /** Output format. */
    fmt: Format;
    /** Output file, or stdout when absent. */
    output?: string | undefined;
};

/**
 * Infers a format from a file extension, defaulting to JSON.
 *
 * Standard streams carry no extension, so they take the default.
 */
export function infer(path: string | undefined): Format {
    if (!path || path === "-") return "json";
    return path.endsWith(".sql") ? "sql" : "json";
}

/** Dump entrypoint. */
export async function main(args: Args) {
    // Open database
    const cxn = db.open(args.db);

    // Serialize collection
    const out =
        args.fmt === "sql"
            ? await statements(cxn)
            : `${JSON.stringify(await payload(cxn), null, 2)}\n`;

    // Write output
    if (args.output && args.output !== "-") await Bun.write(args.output, out);
    else await Bun.write(Bun.stdout, out);
}

/** Builds the JSON dump payload. */
async function payload(cxn: db.Cxn) {
    const media = await records(cxn);
    const items = await ownedGames(cxn);
    const copies = await ownedBooks(cxn);
    return {
        media,
        ...(copies.length ? { books: { owned: copies } } : {}),
        ...(items.length ? { games: { owned: items } } : {}),
    };
}

/** Collects every media record, oldest first. */
async function records(cxn: db.Cxn) {
    const rows = await cxn
        .select({
            id: schema.media.id,
            kind: schema.media.kind,
            created: schema.media.created,
            updated: schema.media.updated,
        })
        .from(schema.media)
        .orderBy(asc(schema.media.created));

    // Load items
    const items = new Map<string, { kind: Kind; item: { id: string } }>();
    for (const decl of ITEMS) {
        for (const item of await cxn.select().from(decl.table)) {
            const row = item as { id: string };
            items.set(row.id, { kind: decl.kind, item: row });
        }
    }

    // Load metadata
    const ids = rows.map((row) => row.id);
    const applied = await tags(cxn, ids);
    const activity = await logs(cxn, ids);

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

/** Collects the owned game items, each with its game references. */
async function ownedGames(cxn: db.Cxn) {
    const rows = await cxn
        .select()
        .from(schema.games_owned)
        .orderBy(asc(schema.games_owned.kind), asc(schema.games_owned.title));
    if (!rows.length) return [];

    const refs = await cxn
        .select({
            owned: schema.games_owned_ref.owned,
            game: schema.games_owned_ref.game,
        })
        .from(schema.games_owned_ref)
        .orderBy(schema.games_owned_ref.owned, schema.games_owned_ref.idx);
    const held = new Map<string, string[]>();
    for (const ref of refs) {
        const games = held.get(ref.owned);
        if (games) games.push(ref.game);
        else held.set(ref.owned, [ref.game]);
    }

    return rows.map((row) => {
        const games = held.get(row.id);
        return games?.length ? { ...row, games } : { ...row };
    });
}

/** Collects the owned book copies. */
async function ownedBooks(cxn: db.Cxn) {
    return cxn
        .select()
        .from(schema.books_owned)
        .orderBy(asc(schema.books_owned.isbn));
}

/**
 * Renders every row of every user table as an idempotent INSERT statement.
 *
 * Table and column names come from the database itself, so this walks
 * `sqlite_master` directly rather than through the query builder.
 */
async function statements(cxn: db.Cxn) {
    const out: string[] = ["BEGIN TRANSACTION;"];

    // List tables
    const tables = await cxn.all<{ name: string }>(
        sql.raw(
            "SELECT name FROM sqlite_master WHERE type = 'table' " +
                "AND name NOT LIKE 'sqlite_%' " +
                "AND name NOT LIKE '\\_\\_%' ESCAPE '\\' " +
                "ORDER BY rowid",
        ),
    );

    for (const { name } of tables) {
        // Inspect columns
        const cols = await cxn.all<{ name: string }>(
            sql.raw(`SELECT name FROM pragma_table_info('${name}')`),
        );
        if (!cols.length) continue;

        // Render rows
        const names = cols.map((col) => col.name).join(", ");
        const quoted = cols
            .map((col) => `quote(${col.name})`)
            .join(" || ', ' || ");
        const rows = await cxn.all<{ stmt: string }>(
            sql.raw(
                `SELECT 'INSERT OR IGNORE INTO "${name}" (${names}) VALUES (' ` +
                    `|| ${quoted} || ');' AS stmt FROM "${name}"`,
            ),
        );
        for (const row of rows) out.push(row.stmt);
    }

    out.push("COMMIT;");
    return `${out.join("\n")}\n`;
}
