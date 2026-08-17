//! Serve subcommand.

import * as cfg from "../app/cfg";
import { log } from "../app/log";
import { build } from "../web";

/** Serve arguments. */
export type Args = cfg.Config & {
    /** SQLite database file. */
    db: string;
    /** Path to configuration file. */
    config: string;
};

/** Default server bind address. */
const HOST = "::1";
/** Default server bind port. */
const PORT = 3000;

/** Serve entrypoint. */
export async function main(args: Args) {
    // Resolve configuration, preferring flags over file values
    const file = await cfg.load(args.config);
    const host = args.host ?? file.host ?? HOST;
    const port = args.port ?? file.port ?? PORT;
    const token = args.token ?? file.token;
    const prefix = args.prefix ?? file.prefix;

    // Warn about read-only mode
    if (!token) {
        log.warn("no API key configured");
        log.warn("running in read-only mode");
    }

    // Serve the collection
    const app = build({ db: args.db, token, prefix });
    app.listen({ hostname: host, port });
    log.info(`listening on ${host}:${port}`);
    return app;
}
