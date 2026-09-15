//! Web server.
//!
//! REST API for managing a personal media collection.
//!
//! Supports books, films, games, links, and television shows. Each kind has its
//! own set of endpoints for listing, fetching, creating, updating, and deleting
//! records. All list endpoints support filtering, sorting, and pagination via
//! query parameters.

import { cors } from "@elysiajs/cors";
import { openapi } from "@elysiajs/openapi";
import { type AnyElysia, Elysia } from "elysia";

import { SCHEMAS } from "../models";
import * as db from "../sql";

import { fail } from "./reply";
import * as books from "./routes/books/index";
import * as films from "./routes/films/index";
import * as games from "./routes/games/index";
import * as media from "./routes/index";
import * as links from "./routes/links/index";
import * as logs from "./routes/logs";
import * as shows from "./routes/shows/index";
import * as tags from "./routes/tags";

/** Server options. */
export type Options = {
    /** SQLite database file. */
    db: string;
    /** Bearer token required for write operations. */
    token?: string | undefined;
    /** URL prefix when served behind a reverse proxy. */
    prefix?: string | undefined;
    /** Directory holding dumped ROMs. */
    roms: string;
};

const TAGS = [
    { name: "media", description: "Any media item." },
    { name: "books", description: "Reading items." },
    { name: "books/owned", description: "Owned book copies." },
    { name: "films", description: "Watched films." },
    { name: "games", description: "Video games." },
    { name: "games/owned", description: "Owned game items." },
    { name: "links", description: "Web bookmarks." },
    { name: "shows", description: "Television shows." },
];

/** Details of a failed validation. */
type Invalid = {
    /** Which part of the request failed: `query`, `params`, or `body`. */
    type?: string;
    /** Every offending value, not just the first. */
    all?: { path?: string; message?: string }[];
};

/**
 * Requests whose body arrived with the wrong media type, against what was
 * expected of them.
 *
 * A parse hook cannot answer the request itself, so the decision is recorded
 * here and acted on when the resulting error surfaces.
 */
const unsupported = new WeakMap<Request, string>();

/** The one route whose body is bytes rather than JSON. */
const OPAQUE = "/games/owned/:id/roms";

const JSON_TYPE = "application/json";
const BYTE_TYPE = "application/octet-stream";

/**
 * Parses a request body.
 *
 * Owning the parse step tells a wrong media type apart from a body that cannot
 * be parsed. A route that takes no body sends none, so the media type is only
 * enforced once a body actually arrives, and an absent body parses as an empty
 * object for the route's own schema to judge.
 *
 * Routes carrying a body additionally name the built-in `json` parser, which
 * never runs but records the media type the spec advertises: a parser given as
 * a function leaves the generator with nothing to name.
 *
 * A ROM upload is the one body this does not read. Its bytes are opaque and
 * arbitrarily large, so the route streams them from the request itself. Leaving
 * them here also means an unauthorized upload is refused before a single chunk
 * has been taken, since the guard runs between this and the handler.
 */
async function parse({ request, route }: { request: Request; route: string }) {
    const kind = request.headers.get("content-type") ?? "";
    if (route === OPAQUE && request.method === "POST") {
        if (!kind.includes(BYTE_TYPE)) {
            unsupported.set(request, BYTE_TYPE);
            throw new Error("unsupported media type");
        }
        return {};
    }
    const text = await request.text();
    if (!text) return {};
    if (!kind.includes(JSON_TYPE)) {
        unsupported.set(request, JSON_TYPE);
        throw new Error("unsupported media type");
    }
    return JSON.parse(text) as unknown;
}

/** Converts a framework error into the API's error response body. */
function handle(ctx: {
    code: unknown;
    error: unknown;
    request: Request;
}) {
    const { code, error, request } = ctx;

    const expected = unsupported.get(request);
    if (expected) {
        return fail(
            "unsupported_media_type",
            `Expected Content-Type: ${expected}.`,
        );
    }
    if (code === "PARSE") {
        return fail("malformed_json", "Request body is not valid JSON.");
    }
    if (code === "VALIDATION") {
        const invalid = error as Invalid;

        // Report every offending value, keyed by JSON pointer
        const fields = (invalid.all ?? [])
            .filter((failed) => failed.message)
            .map((failed) => ({
                path: failed.path || "/",
                message: failed.message!,
            }));

        if (invalid.type === "query") {
            return fail("invalid_query", "Invalid query parameters.", fields);
        }
        if (invalid.type === "params") {
            return fail("invalid_params", "Invalid path parameters.", fields);
        }
        return fail("invalid_body", "Invalid request body.", fields);
    }

    return undefined;
}

/** Builds the application router. */
export function build(opts: Options) {
    const cxn = db.open(opts.db);

    const app = new Elysia()
        .state("token", opts.token)
        .use(cors())
        .use(
            openapi({
                provider: null,
                path: "/openapi",
                documentation: {
                    openapi: "3.1.0",
                    info: {
                        title: "media",
                        description: "Media collection API server.\n",
                        contact: { name: "Zakhary Kaplan", email: "me@zakhary.dev" },
                        license: { name: "MIT OR Apache-2.0" },
                        version: "0.1.0",
                    },
                    tags: TAGS,
                    ...(opts.prefix ? { servers: [{ url: opts.prefix }] } : {}),
                    components: {
                        schemas: SCHEMAS as never,
                        securitySchemes: {
                            BearerAuth: { type: "http", scheme: "bearer" },
                        },
                    },
                },
            }),
        )
        .onParse(parse)
        .onError({ as: "global" }, handle);

    // Serve the spec at `/openapi.json`.
    //
    // The plugin publishes it at `/openapi/json`, while the API contract puts
    // it one path up. Registering it here also keeps the literal path ahead of
    // the catch-all `/:id` route.
    const built: { app?: AnyElysia } = {};
    let spec: unknown;
    const document = new Elysia({ name: "openapi.json" }).get(
        "/openapi.json",
        async ({ request }) => {
            if (!spec) {
                const url = new URL(request.url);
                url.pathname = "/openapi/json";
                const res = await built.app!.handle(new Request(url));
                spec = await res.json();
            }
            return spec;
        },
        { detail: { hide: true } },
    );

    // Mount each kind before the catch-all `/:id` routes
    let routed = app.use(document) as unknown as AnyElysia;
    routed = routed
        .use(books.router(cxn) as never)
        .use(films.router(cxn) as never)
        .use(games.router(cxn, opts.roms) as never)
        .use(links.router(cxn) as never)
        .use(shows.router(cxn) as never);

    // Routes over any kind, which the paths above take precedence over
    const root = {
        prefix: "",
        subject: "a media item",
        noun: "Media",
        tag: "media",
    };
    routed = routed
        .use(media.router(cxn) as never)
        .use(tags.router(cxn, root) as never)
        .use(logs.router(cxn, root) as never);

    built.app = routed;
    return routed;
}
