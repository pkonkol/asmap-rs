# usage

- start top level DB `docker-compose up`
- `rustup target add wasm32-unknown-unknown`

## clearing up the database
Either run `cargo run -- clear-db` or 
run `./cleanup.sh --database --generated --downloaded` and then `docker-compose up`

## fill DB with data

To initialize the database with all available datasources at once
`cargo run -- load-all`

### to update database partially

`cargo run -- load-asrank` will download the data directly from caida's graphql API. Slower but 
    needs no external steps.
`cargo run -- load-asrank -a asns.jsonl` will use file downloaded by official `asrank-download.py`
    from caida website.

#### these two will work only after running load-asrank first
`cargo run -- load-ipnetdb`
`cargo run -- load-stanford-asdb`

## start web service

`cargo run -- start`

### development

`cd asmap && ./dev.sh` This will automatically rebuild the app when code changes are detected.

