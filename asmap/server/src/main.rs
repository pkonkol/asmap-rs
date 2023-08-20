use axum::{routing::get, Router};
use clap::Parser;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use handlers::{as_handler, ws_test_handler};
use state::ServerState;

mod handlers;
mod state;

// const ASDB_CONN_STR: &str = "mongodb://devuser:devpass@localhost:27018/?authSource=asdbmaker";
// const ASDB_DB: &str = "asdbmaker";
const CONFIG_PATH: &str = "../../config.yaml";

// Setup the command line interface with clap.
#[derive(Parser, Debug)]
#[clap(name = "server", about = "A server for our wasm project!")]
struct Opt {
    /// set the log level
    #[clap(short = 'l', long = "log", default_value = "debug")]
    log_level: String,

    /// set the listen addr
    #[clap(short = 'a', long = "addr", default_value = "::1")]
    addr: String,

    /// set the listen port
    #[clap(short = 'p', long = "port", default_value = "8080")]
    port: u16,

    /// set the directory where static files are to be found
    #[clap(long = "static-dir", default_value = "../dist")]
    static_dir: String,
}

#[tokio::main]
async fn main() {
    let cfg = config::parse(CONFIG_PATH);
    let opt = Opt::parse();
    // Setup logging & RUST_LOG from args
    // if std::env::var("RUST_LOG").is_err() {
    //     std::env::set_var("RUST_LOG", format!("{},hyper=info,mio=info", opt.log_level))
    // }
    // enable console logging
    tracing_subscriber::fmt::init();

    let state = ServerState::new(&cfg.mongo_conn_str, &cfg.db_name).await;
    let app = Router::new()
        .route("/ws-test", get(ws_test_handler))
        .route("/as", get(as_handler))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let sock_addr = SocketAddr::from((
        IpAddr::from_str(opt.addr.as_str()).unwrap_or(IpAddr::V6(Ipv6Addr::LOCALHOST)),
        opt.port,
    ));

    log::info!("listening on http://{}", sock_addr);

    axum::Server::bind(&sock_addr)
        .serve(app.into_make_service())
        .await
        .expect("Unable to start server");
}
