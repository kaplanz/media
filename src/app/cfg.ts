//! Application configuration.

import { homedir } from "node:os";
import { join } from "node:path";

import { parse } from "smol-toml";

/** Application configuration data. */
export type Config = {
    /** Server bind address. */
    host?: string;
    /** Server bind port. */
    port?: number;
    /** Bearer token required for write operations. */
    token?: string;
    /** URL prefix when served behind a reverse proxy. */
    prefix?: string;
};

/** An error caused by loading the configuration. */
export class Invalid extends Error {}

/** Returns the path to the application's configuration file. */
export const path = () =>
    join(
        process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config"),
        "media",
        "config.toml",
    );

/**
 * Loads configuration data from a file.
 *
 * A missing file is not an error; defaults fill unset fields.
 */
export async function load(at: string): Promise<Config> {
    const file = Bun.file(at);
    if (!(await file.exists())) return {};
    try {
        return parse(await file.text()) as Config;
    } catch (cause) {
        throw new Invalid("parsing configuration failed", { cause });
    }
}
