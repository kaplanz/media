#!/usr/bin/env bun
//! Media collection API server.

import { program } from "./app/cli";

await program().parseAsync();
