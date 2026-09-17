# media

My media collection, served as a REST API. Built with [Elysia][elysia] on
[Bun][bun], stored in [SQLite][sqlite].

[![License][lic.badge]][lic.hyper]
[![supports Linux][nix.badge]](#)
[![supports macOS][mac.badge]](#)

Tracks books, films, games, links, and television shows, alongside the physical
copies, consoles, and accessories that go with them.

## Usage

Requires [Bun][bun] to be installed.

```sh
bun install             # install dependencies
bun run build           # compile executable to ./dist/media
bun run src/index.ts    # run from source instead
```

### Commands

As a single binary, `media` provides several commands. Run it with `-h` to see
usage information.

- `serve`: starts the HTTP server against the provided database, applying the
  schema first. Every statement is idempotent, so a new file needs no setup and
  an existing one picks up anything added since. Aliased as `s`.
- `dump`: exports the collection to JSON or SQL, printed to the console unless
  `-o` names a file. Aliased as `export`.
- `load`: imports a collection from JSON or SQL, read from the console unless
  `-i` names a file. Aliased as `import`.

Existing records are left untouched on load, so loading the same input twice is
equivalent to loading it once. `-f` selects the format, either `json` (default)
or `sql`, inferred from the file extension when omitted. Since both default to
the standard streams, they compose naturally with pipes:

```sh
media dump -f sql media.db | sqlite3 copy.db
media dump media.db | media load other.db
```

### Configuration

`serve` loads persistent options from the first file found, in order:

1. Command-line option `--config=<PATH>`.
1. Environment variable `MEDIA_CFG`.
1. Default path `$XDG_CONFIG_HOME/media/config.toml`.

```toml
host  = "::1"
port  = 3000
token = "secret"
roms  = "/srv/media/roms"
```

`roms` names the directory holding dumped ROMs, defaulting to
`$XDG_DATA_HOME/media/roms`. Only the metadata lives in the database, so this
directory and the database file are backed up together or not at all.

Where an option is given in several places, the last of cli, env, file wins, so
anything in the file may be overridden at the command line.

> [!NOTE]
>
> The database may also be supplied through the environment as `MEDIA_DB`, in
> which case the positional argument may be omitted.

### Logging

Every command accepts `-v` to raise the logging verbosity and `-q` to lower it,
each repeatable. Records are written to standard error, formatted when attached
to a terminal and emitted as JSON when redirected.

## API

A machine-readable [OpenAPI][openapi] document is served at `/openapi.json`.

Each kind has its own endpoints for listing, fetching, creating, updating, and
deleting records, alongside a unified list across every kind at the root. All
list endpoints support filtering, sorting, and pagination through query
parameters.

Write endpoints (`POST`, `PUT`, `PATCH`, `DELETE`) require the bearer token when
one is configured. Without a token, the server runs read-only. Each operation
declares for itself whether it is guarded, so the requirement the document
advertises is the one the server enforces, and a read may be guarded too: the
ROM bytes below are.

> [!IMPORTANT]
>
> Errors answer with `{ "error": { "code", "message", "fields" } }`, where the
> code is a stable string worth branching on and `fields` names the values at
> fault when the failure is about one.

A book's authors are an ordered sub-resource at `/books/{id}/authors`, mirroring
tags except that position is meaningful: replacing the list fixes the order,
while adding one appends. They are also listed on the book itself as
`item.authors`.

Physical property is recorded apart from the works themselves, since a thing
that is owned and a thing that has been read or played are different facts. Both
wrap the stored columns in an `item` object, beside the records they resolve:

- `/books/owned`: copies, tracked by ISBN so that a copy may be recorded without
  a matching reading entry. That entry is attached as `book` when one exists.
- `/games/owned`: releases, consoles, and accessories, discriminated by `kind`
  and each referencing under `games` the games it carries. The paths
  `/games/owned/releases`, `/games/owned/consoles`, and `/games/owned/extras`
  are shorthand for the corresponding `kind` filter. Write endpoints name those
  games as either `game=<id>` or `games=[<id0>, <id1>, ...]`, the former being
  shorthand for a list of one. `hash=<hex>` narrows the list to the items
  holding a given dump, and each record carries its dumps under `roms`.

Something sold, lost, or given away is still a record worth keeping, so `sold`
carries the date it left and is empty while the thing is still held. Listings
count what you have, so `sold` defaults to `no`, while `sold=yes` lists what
has gone and `sold=any` lists everything. Fetching an item by its identifier
ignores the filter, since a record asked for by name ought to answer. This is
the only filter that constrains by being absent.

A dumped ROM is a fact about the cartridge or disc held rather than about the
game, so dumps hang off the owned item. The database records only the size, the
dump date, and the CRC-32, MD5, SHA-1, and SHA-256 digests, served as lowercase
hexadecimal under `hash`. The bytes themselves live in the store, under
`{roms}/{owned}/{rom}`, named by identifier alone.

Each path says one thing. `/games/owned/{id}/roms` is about ownership, so it
lists what an item holds and accepts what is dumped from it. A dump is keyed by
its own identifier, so `/games/owned/roms/{rom}` addresses one wherever it
belongs, and `/games/owned/roms` lists every dump across the collection,
filtered by `owned=<id>`, `hash=<hex>`, or `q=<text>` against the title. A
hash names its own digest by width, so 8 hexadecimal characters is matched as a
CRC-32, 32 as an MD5, 40 as a SHA-1, and 64 as a SHA-256.

Dumps ignore disposal, since selling a cartridge does not delete the dump taken
from it. The two hash searches therefore answer different questions.
`/games/owned?hash=<hex>` asks whether the thing holding that dump is still
held, and so respects `sold`. `/games/owned/roms?hash=<hex>` asks whether the
dump exists at all, and so does not.

Digests are computed once, when the bytes arrive, so listing costs an index
read rather than a rehash. `present` reports whether the file is still in the
store, since a collection loaded from an export carries every row and none of
the bytes, and `/games/owned/roms/{rom}/verify` rehashes one file and compares
it against what was recorded.

> [!IMPORTANT]
>
> The bytes are the one read this API guards. `/games/owned/roms/{rom}/data`
> requires the bearer token, while the metadata beside it stays open, so a
> collection may be published without publishing what it holds.

Uploads take a raw body, with the title and dump date as optional query
parameters. Note that a body read from standard input is sent as JSON unless
the media type says otherwise:

```sh
xh POST :3000/games/owned/$ID/roms \
   title=='Chrono Trigger (USA).sfc' \
   Content-Type:application/octet-stream \
   Authorization:"Bearer $TOKEN" < 'Chrono Trigger (USA).sfc'
```

## Organization

Source layout follows the shape of the API it serves, so a path on the wire
lands in the file of the same name.

```
./
├── package.json        # project manifest
├── README.md           # this document
├── tsconfig.json       # compiler options
├── sql/                # schema
│  └── main.sql         # data definitions
└── src/                # source files
   ├── index.ts         # entrypoint
   ├── models.ts        # api models
   ├── app/             # application
   │  ├── cfg.ts        # configuration
   │  ├── cli.ts        # argument parsing
   │  └── log.ts        # logging
   ├── exe/             # subcommands
   │  ├── dump.ts       # export
   │  ├── load.ts       # import
   │  └── serve.ts      # http server
   ├── sql/             # database
   │  └── schema.ts     # table definitions
   └── web/             # rest api
      ├── record.ts     # record assembly
      ├── reply.ts      # responses, errors
      └── routes/       # endpoints
         ├── index.ts   # any kind
         ├── item.ts    # per-kind records
         ├── logs.ts    # activity logs
         ├── tags.ts    # tags
         ├── books/     # reading items
         ├── films/     # watched films
         ├── games/     # video games
         ├── links/     # web bookmarks
         └── shows/     # television shows
```

> [!TIP]
>
> `item.ts`, `tags.ts`, and `logs.ts` are generic over a kind's declaration in
> `models.ts`, so each is mounted once per kind rather than copied into one.

## License

This project is dual-licensed under both [MIT License][lic.mit] and [Apache
License 2.0][lic.apache]. You have permission to use this code under the
conditions of either license pursuant to the rights granted by the chosen
license.

<!--
  Reference-style links
-->

[bun]:     https://bun.sh
[elysia]:  https://elysiajs.com
[openapi]: https://www.openapis.org
[sqlite]:  https://sqlite.org

[lic.apache]: ./LICENSE-APACHE
[lic.mit]:    ./LICENSE-MIT

[lic.badge]: https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue
[lic.hyper]: #license
[nix.badge]: https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=000
[mac.badge]: https://img.shields.io/badge/macOS-000?logo=apple&logoColor=fff
