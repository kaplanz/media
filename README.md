# media

[![License][lic.badge]][lic.hyper]

[lic.badge]: https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue
[lic.hyper]: #license

Media collection API server.

A REST API for managing a personal media library. Tracks books, films,
games, links, and television shows. Records are stored in SQLite, served
over HTTP with optional bearer-token authentication, and documented via
an auto-generated OpenAPI spec with a Scalar UI.

## Build

```sh
cargo build --release
```

## Usage

```
media serve [OPTIONS] <DATABASE>
```

### Configuration

Configuration is loaded with the following precedence:

1. Command-line flags (`--host`, `--port`, `--token`).
2. Environment variables (`MEDIA_HOST`, `MEDIA_PORT`, `MEDIA_TOKEN`).
3. Config file (`--config`, env `MEDIA_CFG`, default:
   `~/.config/media/config.toml`).

Config file format:

```toml
host  = "::1"
port  = 3000
token = "secret"
```

### API

All endpoints are documented at `/docs`. A machine-readable OpenAPI spec
is available at `/openapi.json`. The API covers books, films, games,
links, and shows with full CRUD support, plus tag management and a
unified media list across all kinds.

Write endpoints (`POST`, `PUT`, `DELETE`) require the bearer token if
one is configured.

## License

Licensed under either of [MIT][lic.mit] or [Apache 2.0][lic.apache],
at your option.

[lic.mit]:    LICENSE-MIT
[lic.apache]: LICENSE-APACHE
