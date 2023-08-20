# usage

- start top level DB `docker-compose up`
- run `./prepare.sh` to download necessary files and set up env
- run `./cleanup.sh` when needed
- `rustup target add wasm32-unknown-unknown`

## fill DB with data

`./cargo run -- --init`
or something like this

## start web service

