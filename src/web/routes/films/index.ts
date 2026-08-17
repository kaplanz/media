//! Film routes.
//!
//! Everything under `/films`: the watched films, their tags and logs.

import { Elysia } from "elysia";

import { BY_KIND } from "../../../models";
import type * as db from "../../../sql";
import * as item from "../item";
import * as logs from "../logs";
import * as tags from "../tags";

const decl = BY_KIND.get("film")!;
const mount = tags.mounted(decl);

/** Serves everything under `/films`. */
export const router = (cxn: db.Cxn) =>
    new Elysia({ name: "films" })
        .use(item.router(cxn, decl) as never)
        .use(tags.router(cxn, mount) as never)
        .use(logs.router(cxn, mount) as never);
