//! Link routes.
//!
//! Everything under `/links`: the bookmarks, their tags and logs.

import { Elysia } from "elysia";

import { BY_KIND } from "../../../models";
import type * as db from "../../../sql";
import * as item from "../item";
import * as logs from "../logs";
import * as tags from "../tags";

const decl = BY_KIND.get("link")!;
const mount = tags.mounted(decl);

/** Serves everything under `/links`. */
export const router = (cxn: db.Cxn) =>
    new Elysia({ name: "links" })
        .use(item.router(cxn, decl) as never)
        .use(tags.router(cxn, mount) as never)
        .use(logs.router(cxn, mount) as never);
