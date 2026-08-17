//! API models.
//!
//! Each media kind declares its columns once, with descriptions and
//! nullability. The item, request body, and partial request body schemas are
//! all derived from that declaration: a nullable column may be omitted from a
//! request body, while a non-nullable one is required.

import type { TOptional, TString } from "@sinclair/typebox";
import type { SQLiteColumn, SQLiteTable } from "drizzle-orm/sqlite-core";
import { t, type TSchema } from "elysia";

import * as schema from "./sql/schema";

/** A set of named column schemas. */
export type Fields = Record<string, TSchema>;

/** Schemas published under `components/schemas`, keyed by name. */
export const SCHEMAS: Record<string, TSchema> = {};

/**
 * Publishes a schema under a name, yielding a reference to it.
 *
 * Naming a shape once keeps the document readable where it is used many times,
 * and gives a generated client a type to name rather than an anonymous object.
 */
export function define(name: string, schema: TSchema) {
    SCHEMAS[name] = schema;
    return t.Unsafe<never>({ $ref: `#/components/schemas/${name}` });
}

export const uuid = (about: string) =>
    t.String({ format: "uuid", description: about });
export const str = (about: string) => t.String({ description: about });
export const int = (about: string) => t.Integer({ description: about });
export const bool = (about: string) => t.Boolean({ description: about });

export const nullable = <T extends TSchema>(schema: T, about: string) =>
    t.Union([schema, t.Null()], { description: about });

export const list = <T extends TSchema>(schema: T, about: string) =>
    t.Array(schema, { description: about });

/** Path identifier, rejected with 400 when malformed. */
export const ident = t.String({
    format: "uuid",
    description: "Unique identifier.",
});

/**
 * Declares a closed set of query values.
 *
 * Written as a union of literals because `t.UnionEnum` injects a default equal
 * to its first member, which would silently change how an absent parameter
 * behaves.
 */
export const choice = (values: readonly string[], about: string) =>
    t.Optional(
        t.Union(
            values.map((value) => t.Literal(value)),
            { description: about },
        ),
    ) as unknown as TOptional<TString>;

/** Pagination parameters, shared by every list endpoint. */
export const page = {
    order: t.Optional(
        t.Union([t.Literal("asc"), t.Literal("desc")], {
            description: "Sort direction.",
        }),
    ),
    limit: t.Optional(t.Integer({ description: "Maximum number of results." })),
    offset: t.Optional(t.Integer({ description: "Number of results to skip." })),
};

/**
 * Derives the item, body, and patch schemas for a set of columns.
 *
 * Assembled fields join the item but not the request bodies, since they are
 * written through their own sub-resource rather than with the item.
 */
export function derive(fields: Fields, about: string, extra: Fields = {}) {
    const { id: _, ...rest } = fields;
    const body: Fields = {};
    for (const [name, schema] of Object.entries(rest)) {
        body[name] = "anyOf" in schema ? t.Optional(schema) : schema;
    }

    return {
        item: t.Object({ ...fields, ...extra }, { description: about }),
        body: t.Object(body, { description: "Request body." }),
        patch: t.Partial(t.Object(rest), {
            description: "Partial request body.",
        }),
        columns: Object.keys(rest),
    };
}

/** Capitalizes a word so it reads as part of an identifier. */
export const pascal = (word: string) => word[0]!.toUpperCase() + word.slice(1);

/** Media kind. */
export const KINDS = ["book", "film", "game", "link", "show"] as const;

export type Kind = (typeof KINDS)[number];

/** A table keyed by media identifier. */
export type Keyed = SQLiteTable & { id: SQLiteColumn };

/** Declaration of one media kind. */
type Decl = {
    /** Kind discriminant. */
    kind: Kind;
    /** OpenAPI tag, and the path this kind is mounted under. */
    tag: string;
    /** Sentence fragment naming one item, such as `a book`. */
    one: string;
    /** Sentence fragment naming several items, such as `books`. */
    many: string;
    /** Item description. */
    about: string;
    /** Table holding this kind's columns. */
    table: Keyed;
    /** Column searched by the `q` parameter. */
    search: SQLiteColumn;
    /** Columns available to the `sort` parameter. */
    sort: Record<string, SQLiteColumn>;
    /** Column declarations, in wire order. */
    fields: Fields;
    /** Item fields assembled from related tables, trailing the columns. */
    extra?: Fields;
};

const decl = (spec: Decl) => {
    const derived = derive(spec.fields, spec.about, spec.extra);
    return {
        ...spec,
        ...derived,
        item: define(pascal(spec.kind), derived.item),
    };
};

/** Every media kind, in the order they are mounted. */
export const ITEMS = [
    decl({
        kind: "book",
        tag: "books",
        one: "a book",
        many: "books",
        about: "Reading item.",
        table: schema.books,
        search: schema.books.title,
        sort: {
            title: schema.books.title,
            created: schema.media.created,
            updated: schema.media.updated,
        },
        fields: {
            id: uuid("Unique identifier."),
            isbn: nullable(t.String(), "ISBN-13."),
            hcid: nullable(t.Integer(), "Hardcover ID."),
            title: str("Title."),
            cover: nullable(t.String(), "Cover image URL."),
            about: nullable(t.String(), "Description."),
            color: nullable(t.String(), "Accent color."),
        },
        extra: {
            authors: list(t.String(), "Authors, in listed order."),
        },
    }),
    decl({
        kind: "film",
        tag: "films",
        one: "a film",
        many: "films",
        about: "Watched film.",
        table: schema.films,
        search: schema.films.title,
        sort: {
            title: schema.films.title,
            year: schema.films.year,
            rating: schema.films.rating,
            created: schema.media.created,
            updated: schema.media.updated,
        },
        fields: {
            id: uuid("Unique identifier."),
            tmdb: nullable(t.Integer(), "TMDB ID."),
            title: str("Title."),
            year: nullable(t.Integer(), "Release year."),
            rating: nullable(t.Integer(), "Rating (1-5)."),
        },
    }),
    decl({
        kind: "game",
        tag: "games",
        one: "a game",
        many: "games",
        about: "Video game.",
        table: schema.games,
        search: schema.games.title,
        sort: {
            title: schema.games.title,
            rating: schema.games.rating,
            created: schema.media.created,
            updated: schema.media.updated,
        },
        fields: {
            id: uuid("Unique identifier."),
            title: str("Title."),
            platform: nullable(t.String(), "Platform."),
            rating: nullable(t.Integer(), "Rating (1-5)."),
        },
    }),
    decl({
        kind: "link",
        tag: "links",
        one: "a link",
        many: "links",
        about: "Web bookmark.",
        table: schema.links,
        search: schema.links.title,
        sort: {
            title: schema.links.title,
            created: schema.media.created,
            updated: schema.media.updated,
        },
        fields: {
            id: uuid("Unique identifier."),
            url: str("URL."),
            title: nullable(t.String(), "Title."),
        },
    }),
    decl({
        kind: "show",
        tag: "shows",
        one: "a show",
        many: "shows",
        about: "Television show.",
        table: schema.shows,
        search: schema.shows.title,
        sort: {
            title: schema.shows.title,
            year: schema.shows.year,
            rating: schema.shows.rating,
            created: schema.media.created,
            updated: schema.media.updated,
        },
        fields: {
            id: uuid("Unique identifier."),
            tmdb: nullable(t.Integer(), "TMDB ID."),
            title: str("Title."),
            year: nullable(t.Integer(), "First air year."),
            rating: nullable(t.Integer(), "Rating (1-5)."),
        },
    }),
];

export type Item = (typeof ITEMS)[number];

/** Every media kind, keyed by discriminant. */
export const BY_KIND = new Map<Kind, Item>(
    ITEMS.map((item) => [item.kind, item]),
);

const ACTIVITY = ["start", "stop", "done"] as const;

/** Activity kind, as the document shows it. */
const Activity = t.Unsafe<never>({
    description: "Activity kind.",
    enum: [...ACTIVITY],
});

/** Activity kind, as a request body validates it. */
const activity = t.Union(
    ACTIVITY.map((kind) => t.Literal(kind)),
    { description: "Activity kind." },
);

/** Item metadata. */
export const Meta = define(
    "Meta",
    t.Object(
        {
            created: int("Created timestamp (Unix seconds)."),
            updated: int("Updated timestamp (Unix seconds)."),
        },
        { description: "Item metadata." },
    ),
);

/** Activity log. */
export const Log = define(
    "Log",
    t.Object(
        {
            id: uuid("Unique identifier."),
            kind: Activity,
            date: int("Activity date (Unix seconds)."),
        },
        { description: "Activity log." },
    ),
);

/** Activity log request body. */
export const Body = t.Object(
    {
        kind: activity,
        date: t.Optional(
            nullable(t.Integer(), "Activity date (Unix seconds); defaults to now."),
        ),
    },
    { description: "Request body." },
);

/** Where the item schema of one kind sits in the document. */
const located = (kind: Kind) => `#/components/schemas/${pascal(kind)}`;

/**
 * A media record, whatever kind it wraps.
 *
 * `item` holds the columns of the record's own kind, so the two fields are not
 * independent: the conditionals tie each `kind` to the one item type it admits.
 * OpenAPI has no generics to write `Record<Book>` with, and stating the
 * constraint this way is the closest the schema language comes to it.
 *
 * Exported as `Media`, since `Record` names a TypeScript built-in.
 */
export const Media = define(
    "Record",
    t.Unsafe<never>({
        description: "Media record.",
        type: "object",
        required: ["kind", "item", "meta", "logs", "tags"],
        properties: {
            kind: { description: "Media kind.", enum: [...KINDS] },
            item: {
                // Films and shows carry identical columns, so this cannot be
                // `oneOf`: such an item matches two branches. The conditionals
                // below are what pin it to exactly one.
                description: "The item this record wraps.",
                anyOf: KINDS.map((kind) => ({ $ref: located(kind) })),
            },
            meta: { $ref: "#/components/schemas/Meta" },
            logs: {
                description: "Activity logs.",
                type: "array",
                items: { $ref: "#/components/schemas/Log" },
            },
            tags: {
                description: "Applied tags.",
                type: "array",
                items: { type: "string" },
            },
        },
        allOf: KINDS.map((kind) => ({
            if: { properties: { kind: { const: kind } }, required: ["kind"] },
            then: { properties: { item: { $ref: located(kind) } } },
        })),
    }),
);

/**
 * The record of one kind, as a record narrowed to that kind.
 *
 * Fixing `kind` is enough: the conditionals above then admit only that kind's
 * item, so the narrowing states what varies and nothing more.
 */
export const narrowed = (kind: Kind) =>
    t.Unsafe<never>({
        description: `Media record wrapping a ${kind}.`,
        allOf: [
            { $ref: "#/components/schemas/Record" },
            { properties: { kind: { const: kind } } },
        ],
    });
