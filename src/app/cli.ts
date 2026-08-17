//! Command-line interface.

import { Command, Option } from "@commander-js/extra-typings";

import * as dump from "../exe/dump";
import * as load from "../exe/load";
import * as serve from "../exe/serve";

import * as cfg from "./cfg";
import { init } from "./log";

const DATABASE = "SQLite database file [env: MEDIA_DB]";

/** Resolves the database path, which may come from the environment instead. */
function database(cmd: { error(msg: string): never }, given?: string) {
    const url = given ?? process.env.MEDIA_DB;
    if (!url) cmd.error("error: missing required argument 'database'");
    return url;
}

/** Counts each occurrence of a repeated flag. */
const count = (_: string, prior: number) => prior + 1;

const format = (verb: string) =>
    new Option("-f, --format <format>", `${verb} format`).choices(dump.FORMATS);

/** Builds the command tree, from which usage and help are generated. */
export function program() {
    const media = new Command("media")
        .description("Serve a media collection over HTTP")
        .version("0.1.0", "-V, --version")
        .showHelpAfterError()
        .hook("preAction", (_, action) => {
            const opts = action.opts();
            init(Number(opts.verbose) - Number(opts.quiet));
        });

    media
        .command("serve", { isDefault: true })
        .alias("s")
        .description("Start the HTTP server")
        .argument("[database]", DATABASE)
        .addOption(
            new Option("--config <path>", "Path to configuration file")
                .env("MEDIA_CFG")
                .default(cfg.path()),
        )
        .addOption(new Option("--host <host>", "Server bind address").env("HOST"))
        .addOption(
            new Option("--port <port>", "Server bind port")
                .env("PORT")
                .argParser(Number),
        )
        .addOption(
            new Option(
                "--token <token>",
                "Bearer token required for write operations",
            ).env("TOKEN"),
        )
        .addOption(
            new Option(
                "--prefix <prefix>",
                "URL prefix when behind a reverse proxy",
            ).env("PREFIX"),
        )
        .option("-v, --verbose", "Increase logging verbosity", count, 0)
        .option("-q, --quiet", "Decrease logging verbosity", count, 0)
        .action(async (given, opts, cmd) => {
            await serve.main({ ...opts, db: database(cmd, given) });
        });

    media
        .command("dump")
        .alias("export")
        .description("Export the media collection")
        .argument("[database]", DATABASE)
        .addOption(format("Output"))
        .option("-o, --output <path>", "Output file (default: stdout)")
        .option("-v, --verbose", "Increase logging verbosity", count, 0)
        .option("-q, --quiet", "Decrease logging verbosity", count, 0)
        .action(async (given, opts, cmd) => {
            await dump.main({
                db: database(cmd, given),
                fmt: opts.format ?? dump.infer(opts.output),
                output: opts.output,
            });
        });

    media
        .command("load")
        .alias("import")
        .description("Import the media collection")
        .argument("[database]", DATABASE)
        .addOption(format("Input"))
        .option("-i, --input <path>", "Input file (default: stdin)")
        .option("-v, --verbose", "Increase logging verbosity", count, 0)
        .option("-q, --quiet", "Decrease logging verbosity", count, 0)
        .action(async (given, opts, cmd) => {
            await load.main({
                db: database(cmd, given),
                fmt: opts.format ?? dump.infer(opts.input),
                input: opts.input,
            });
        });

    return media;
}
