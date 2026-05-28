# usage

- start top level DB `docker-compose up`
- `rustup target add wasm32-unknown-unknown`

## clearing up the database
Either run `cargo run -p asmap-cli -- clear-db` or
run `./cleanup.sh --database --generated --downloaded` and then `docker-compose up`

## fill DB with data

For developer quickstart use the command provided below import from asmap.jsonl.gz.

To initialize the database with all available datasources at once
`cargo run -p asmap-cli -- load-all`

### to update database partially

`cargo run -p asmap-cli -- load-asrank` will download the data directly from caida's graphql API. Slower but
    needs no external steps.
`cargo run -p asmap-cli -- load-asrank -a asns.jsonl` will use file downloaded by official `asrank-download.py`
    from caida website.

#### these two will work only after running load-asrank first
`cargo run -p asmap-cli -- load-ipnetdb`
`cargo run -p asmap-cli -- load-stanford-asdb`

## start web service

`cargo run -p asmap-cli -- start`

This builds and serves the React frontend in `asmap/frontend-ts`.

## export/import database (jsonl.gz)

Export the full database (all collections) to a compressed JSONL file:

`cargo run -p asmap-cli -- export-db -o asmap.jsonl.gz`

Import from an exported file:

`cargo run -p asmap-cli -- import-db -i asmap.jsonl.gz`

WARNING: `import-db` drops existing collections before importing.

### development

`cd asmap && ./dev.sh` This will automatically rebuild the app when code changes are detected.

## additionals

Run this in asdb-builder/src/asrank if the schema gets outdated
`cargo install graphql_client_cli --force`
`graphql-client introspect-schema https://api.asrank.caida.org/v2/graphql > schema.json`