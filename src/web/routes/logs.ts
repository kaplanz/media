//! Activity log routes.

import { and, eq } from "drizzle-orm";
import { Elysia, t } from "elysia";

import { Body, ident, Log } from "../../models";
import * as db from "../../sql";
import * as schema from "../../sql/schema";
import { exists } from "../record";
import { bare, fail, failed, json, operation } from "../reply";

import { type Mount, ops } from "./tags";

export function router(cxn: db.Cxn, mount: Mount) {
    const { prefix, subject, noun, kind, tag } = mount;
    const thing = noun.toLowerCase();
    const id = ops(noun, "Logs", "Log");
    const activity = json(t.Array(Log));
    const params = t.Object({ id: ident });
    const target = t.Object({ id: ident, log: ident });

    /** Rejects a request for a media item that does not exist. */
    const guard = async (media: string) =>
        (await exists(cxn, media, kind))
            ? undefined
            : fail("not_found", `No ${thing} with that ID.`);

    /** Loads the logs of one media item, ordered by date. */
    const load = (media: string) =>
        cxn
            .select({
                id: schema.logs.id,
                kind: schema.logs.kind,
                date: schema.logs.date,
            })
            .from(schema.logs)
            .where(eq(schema.logs.media, media))
            .orderBy(schema.logs.date);

    /** Builds a log row, defaulting its date to now. */
    const entry = (media: string, body: { kind: string; date?: unknown }) => ({
        id: crypto.randomUUID(),
        media,
        kind: body.kind,
        date: (body.date as number | null | undefined) ?? db.timestamp(),
    });

    return new Elysia({ prefix, name: `logs:${tag}:${prefix}` })
        .get(
            "/:id/logs",
            async ({ params: args }) =>
                (await guard(args.id)) ?? load(args.id),
            {
                params,
                detail: operation({
                    tag,
                    id: id.list,
                    about: `List logs for ${subject}.`,
                    responses: { 200: activity, 404: failed },
                }),
            },
        )
        .put(
            "/:id/logs",
            async ({ params: args, body }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                cxn.transaction((tx) => {
                    tx.delete(schema.logs)
                        .where(eq(schema.logs.media, args.id))
                        .run();
                    for (const log of body) {
                        tx.insert(schema.logs).values(entry(args.id, log)).run();
                    }
                });
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params,
                body: t.Array(Body),
                parse: "json",
                detail: operation({
                    tag,
                    id: id.set,
                    about: `Replace logs for ${subject}.`,
                    write: true,
                    responses: { 200: activity, 404: failed },
                }),
            },
        )
        .post(
            "/:id/logs",
            async ({ params: args, body }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                await cxn.insert(schema.logs).values(entry(args.id, body));
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params,
                body: Body,
                parse: "json",
                detail: operation({
                    tag,
                    id: id.insert,
                    about: `Add a log to ${subject}.`,
                    write: true,
                    responses: { 200: activity, 404: failed },
                }),
            },
        )
        .delete(
            "/:id/logs/:log",
            async ({ params: args }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                const res = await cxn
                    .delete(schema.logs)
                    .where(
                        and(
                            eq(schema.logs.media, args.id),
                            eq(schema.logs.id, args.log),
                        ),
                    );
                if (!db.affected(res)) {
                    return fail("not_found", "No such log on this item.");
                }
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params: target,
                detail: operation({
                    tag,
                    id: id.remove,
                    about: `Remove a log from ${subject}.`,
                    write: true,
                    responses: { 200: activity, 404: failed },
                }),
            },
        );
}
