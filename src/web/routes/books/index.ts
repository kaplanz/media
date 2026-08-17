//! Book routes.
//!
//! Everything under `/books`: the reading items themselves, their tags and
//! logs, their authors, and the copies owned.

import { Elysia } from "elysia";

import { BY_KIND } from "../../../models";
import type * as db from "../../../sql";
import * as item from "../item";
import * as logs from "../logs";
import * as tags from "../tags";

import * as authors from "./authors";
import * as owned from "./owned";

const decl = BY_KIND.get("book")!;
const mount = tags.mounted(decl);

/** Serves everything under `/books`. */
export const router = (cxn: db.Cxn) =>
    new Elysia({ name: "books" })
        .use(item.router(cxn, decl) as never)
        .use(tags.router(cxn, mount) as never)
        .use(logs.router(cxn, mount) as never)
        .use(authors.router(cxn) as never)
        .use(owned.router(cxn) as never);
