use axum::{routing::get, Router};
use clap::Parser;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

use handlers::as_handler;
use state::ServerState;

mod handlers;
mod state;

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

    /// config path
    #[clap(short = 'c', long = "config", default_value = "config.yaml")]
    config: String,

    /// set the directory where static files are to be found
    #[clap(long = "static-dir", default_value = "../dist")]
    static_dir: String,
}

#[tokio::main]
async fn main() {
    let opt = Opt::parse();
    let cfg = config::parse(&opt.config);
    tracing_subscriber::fmt::init();

    let state = ServerState::new(&cfg.mongo_conn_str, &cfg.db_name).await;
    let app = Router::new()
        .route("/as", get(as_handler))
        .fallback_service(ServeDir::new(opt.static_dir))
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
