//! Response types and documentation.

import { type DocumentDecoration, t, type TSchema } from "elysia";

import { define } from "../models";

/** Why a request failed, and the status each reason answers with. */
export const CODES = {
    invalid_query: 400,
    invalid_params: 400,
    malformed_json: 400,
    unauthorized: 401,
    not_found: 404,
    unsupported_media_type: 415,
    invalid_body: 422,
} as const;

export type Code = keyof typeof CODES;

/** One value a request got wrong. */
export type Field = {
    /** JSON pointer to the offending value. */
    path: string;
    /** What was wrong with it. */
    message: string;
};

export const NO_CONTENT = 204;
export const CREATED = 201;

/** A response carrying only a status. */
export const empty = (status: number) => new Response(null, { status });

/** A JSON identifier response, as returned when creating a record. */
export const created = (id: string) =>
    new Response(JSON.stringify(id), {
        status: CREATED,
        headers: { "content-type": "application/json" },
    });

/**
 * Builds an error response, answering with the status its code implies.
 *
 * The code is what a client branches on; the message is for a reader, and the
 * fields say which values were at fault when the reason is a bad value.
 */
export function fail(code: Code, message: string, fields: Field[] = []) {
    const error = { code, message, ...(fields.length ? { fields } : {}) };
    return new Response(JSON.stringify({ error }), {
        status: CODES[code],
        headers: { "content-type": "application/json" },
    });
}

/** A documented JSON response body. */
export const json = (schema: TSchema) => ({
    description: "",
    content: { "application/json": { schema: schema as never } },
});

/** A documented response carrying only a status. */
export const bare = { description: "" };

/** Identifier returned when a record is created. */
export const Id = t.String({ format: "uuid" });

/**
 * Reported error.
 *
 * Bound as `Reason` here, since `Error` names a JavaScript global.
 */
const Reason = define(
    "Error",
    t.Object(
        {
            code: t.Unsafe<never>({
                description: "Stable, machine-readable reason.",
                enum: Object.keys(CODES),
            }),
            message: t.String({ description: "Human-readable summary." }),
            fields: t.Optional(
                t.Array(
                    t.Object(
                        {
                            path: t.String({
                                description:
                                    "JSON pointer to the offending value.",
                            }),
                            message: t.String({
                                description: "What was wrong with it.",
                            }),
                        },
                        { description: "One value a request got wrong." },
                    ),
                    {
                        description:
                            "Values at fault, when the reason is one.",
                    },
                ),
            ),
        },
        { description: "Reported error." },
    ),
);

/** Error response body. */
export const Failure = t.Object(
    { error: Reason },
    { description: "Error response." },
);

/** A documented error response. */
export const failed = json(Failure);

const secured = [{ BearerAuth: [] as string[] }];

/**
 * Documents one operation.
 *
 * Response schemas are declared here rather than through Elysia's `response`
 * option: that option also constrains what a handler may return, which rules
 * out the bare status responses this API uses. Every write is answered with a
 * 401 when the token is absent, so that response is documented for all of them.
 */
export function operation(spec: {
    tag: string;
    id: string;
    about: string;
    responses: NonNullable<DocumentDecoration["responses"]>;
    write?: boolean;
}): DocumentDecoration {
    return {
        tags: [spec.tag],
        operationId: spec.id,
        description: spec.about,
        ...(spec.write ? { security: secured } : {}),
        responses: {
            ...spec.responses,
            ...(spec.write ? { 401: failed } : {}),
        },
    };
}
