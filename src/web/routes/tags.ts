//! Tag routes.
//!
//! Mounted once at the root for any media item, and once under each kind so
//! that the kind constrains the lookup and the operations are documented under
//! that kind's tag.

import { and, eq } from "drizzle-orm";
import { Elysia, t } from "elysia";

import { ident, type Item, type Kind, pascal } from "../../models";
import * as db from "../../sql";
import * as schema from "../../sql/schema";
import { exists } from "../record";
import { bare, fail, failed, json, operation } from "../reply";

/** Where a sub-resource is mounted, and how it is documented there. */
export type Mount = {
    /** Path prefix, empty at the root. */
    prefix: string;
    /** Sentence fragment naming the subject, such as `a book`. */
    subject: string;
    /** Identifier fragment naming the subject, such as `Book`. */
    noun: string;
    /** Kind constraint, absent at the root. */
    kind?: Kind;
    /** OpenAPI tag. */
    tag: string;
};

/** Where a kind's sub-resources hang off it. */
export const mounted = (decl: Item): Mount => ({
    prefix: `/${decl.tag}`,
    subject: decl.one,
    noun: pascal(decl.kind),
    kind: decl.kind,
    tag: decl.tag,
});

/**
 * Builds the four operation IDs of a sub-resource.
 *
 * Operation IDs are global, so each is qualified by both the subject it hangs
 * off and the sub-resource itself, giving `listBookTags` and `insertBookTag`.
 */
export const ops = (noun: string, many: string, one: string) => ({
    list: `list${noun}${many}`,
    set: `set${noun}${many}`,
    insert: `insert${noun}${one}`,
    remove: `remove${noun}${one}`,
});

export function router(cxn: db.Cxn, mount: Mount) {
    const { prefix, subject, noun, kind, tag } = mount;
    const thing = noun.toLowerCase();
    const id = ops(noun, "Tags", "Tag");
    const labels = json(t.Array(t.String()));
    const params = t.Object({ id: ident });
    const target = t.Object({
        id: ident,
        tag: t.String({ description: "Tag label." }),
    });

    /** Rejects a request for a media item that does not exist. */
    const guard = async (media: string) =>
        (await exists(cxn, media, kind))
            ? undefined
            : fail("not_found", `No ${thing} with that ID.`);

    /** Loads the tag labels of one media item. */
    const load = async (media: string) => {
        const rows = await cxn
            .select({ label: schema.tags.label })
            .from(schema.tags)
            .where(eq(schema.tags.media, media))
            .orderBy(schema.tags.label);
        return rows.map((row) => row.label);
    };

    return new Elysia({ prefix, name: `tags:${tag}:${prefix}` })
        .get(
            "/:id/tags",
            async ({ params: args }) =>
                (await guard(args.id)) ?? load(args.id),
            {
                params,
                detail: operation({
                    tag,
                    id: id.list,
                    about: `List tags for ${subject}.`,
                    responses: { 200: labels, 404: failed },
                }),
            },
        )
        .put(
            "/:id/tags",
            async ({ params: args, body }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                cxn.transaction((tx) => {
                    tx.delete(schema.tags)
                        .where(eq(schema.tags.media, args.id))
                        .run();
                    for (const label of body) {
                        tx.insert(schema.tags)
                            .values({ media: args.id, label })
                            .onConflictDoNothing()
                            .run();
                    }
                });
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params,
                body: t.Array(t.String()),
                parse: "json",
                detail: operation({
                    tag,
                    id: id.set,
                    about: `Replace tags for ${subject}.`,
                    write: true,
                    responses: { 200: labels, 404: failed },
                }),
            },
        )
        .put(
            "/:id/tags/:tag",
            async ({ params: args }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                await cxn
                    .insert(schema.tags)
                    .values({ media: args.id, label: args.tag })
                    .onConflictDoNothing();
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params: target,
                detail: operation({
                    tag,
                    id: id.insert,
                    about: `Add a tag to ${subject}.`,
                    write: true,
                    responses: { 200: labels, 404: failed },
                }),
            },
        )
        .delete(
            "/:id/tags/:tag",
            async ({ params: args }) => {
                const missing = await guard(args.id);
                if (missing) return missing;

                const res = await cxn
                    .delete(schema.tags)
                    .where(
                        and(
                            eq(schema.tags.media, args.id),
                            eq(schema.tags.label, args.tag),
                        ),
                    );
                if (!db.affected(res)) {
                    return fail("not_found", "No such tag on this item.");
                }
                await db.touch(cxn, args.id);

                return load(args.id);
            },
            {
                params: target,
                detail: operation({
                    tag,
                    id: id.remove,
                    about: `Remove a tag from ${subject}.`,
                    write: true,
                    responses: { 200: labels, 404: failed },
                }),
            },
        );
}
