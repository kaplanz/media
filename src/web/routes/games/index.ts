//! Game routes.
//!
//! Everything under `/games`: the games themselves, their tags and logs, and
//! the releases, consoles and extras owned.

import { Elysia } from "elysia";

import { BY_KIND } from "../../../models";
import type * as db from "../../../sql";
import * as item from "../item";
import * as logs from "../logs";
import * as tags from "../tags";

import * as owned from "./owned";

const decl = BY_KIND.get("game")!;
const mount = tags.mounted(decl);

/** Serves everything under `/games`. */
export const router = (cxn: db.Cxn) =>
    new Elysia({ name: "games" })
        .use(item.router(cxn, decl) as never)
        .use(tags.router(cxn, mount) as never)
        .use(logs.router(cxn, mount) as never)
        .use(owned.router(cxn) as never);
