use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr};
use std::process::{Command, Stdio};

use asdb_builder::AsdbBuilder;
use clap::{Args, Parser, Subcommand};

const INPUTS_PATH: &str = "inputs";
const ASNS_JSONL: &str = "asns.jsonl";
const SERVER_DEV_SCRIPT: &str = "./dev.sh";
const CONFIG_PATH: &str = "config.yaml";

#[derive(Parser)]
#[command(name = "asmap")]
#[command(about = "Cli for managing asmap data and server")]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(short, long, default_value = INPUTS_PATH)]
    pub iputs_path: String,
}

#[derive(Subcommand)]
enum Commands {
    /// resets the database to starting state
    ClearDB,
    /// Performs all of the loading steps in the correct order
    LoadAll(LoadAllArgs),
    /// Load asrank asns.jsonl from file, takes in the path
    /// todo default path value to inputs
    LoadAsrank(LoadAsrankAsnsArgs),
    /// Downloads if not found and loads IpnetDB data
    LoadIpnetdb,
    /// Downloads and loads into databse the AS categories data from stanford asdb
    LoadStanfordAsdb,
    /// Starts a server with the map
    Start(StartServerArgs),
    // Todo LoadWhois (for range?), LoadIpnetDB, Georesolve(Persons|Orgs|Somethin else?)
}

#[derive(Args)]
struct LoadAsrankAsnsArgs {
    #[arg(short, long)]
    pub asns_filename: Option<String>,
}

#[derive(Args)]
struct LoadAllArgs {
    #[arg(short, long)]
    pub asrank_asns_filename: Option<String>,
}

#[derive(Args)]
struct StartServerArgs {
    #[arg(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(127,0,0,1)))]
    pub ip: IpAddr,
    #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..), default_value_t = 8080)]
    pub port: u16,
}

#[tokio::main]
async fn main() {
    let cfg = config::parse(CONFIG_PATH);
    let args = Cli::parse();

    match args.command {
        Commands::ClearDB => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.iputs_path)
                .await
                .unwrap();
            m.clear_database().await.unwrap();
        }
        Commands::LoadAll(a) => {
            println!("performing complete database load");
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.iputs_path)
                .await
                .unwrap();
            m.load_asrank_asns(a.asrank_asns_filename).await.unwrap();
            m.load_stanford_asdb().await.unwrap();
            m.load_ipnetdb().await.unwrap();
            //let b1 = m.load_stanford_asdb();
            //let b2 = m.load_ipnetdb();
            //let x = futures::future::join_all(vec![b1, b2]).await;
        }
        Commands::LoadAsrank(a) => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.iputs_path)
                .await
                .unwrap();
            let result = m.load_asrank_asns(a.asns_filename).await;
            println!("import result: {result:?}");
        }
        Commands::LoadStanfordAsdb => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.iputs_path)
                .await
                .unwrap();
            m.load_stanford_asdb().await.unwrap();
        }
        Commands::LoadIpnetdb => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.iputs_path)
                .await
                .unwrap();
            m.load_ipnetdb().await.unwrap();
        }
        Commands::Start(_a) => {
            // TODO pass ip and port from `a`
            // TODO fix server not talking with frontend
            // TODO don't rely on dev.sh
            let mut cmd = Command::new(SERVER_DEV_SCRIPT)
                .current_dir("./asmap")
                .stdout(Stdio::piped())
                .spawn()
                .expect("dev script failed");
            let stdout = cmd.stdout.as_mut().unwrap();
            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            for line in stdout_lines {
                let l = line.unwrap();
                println!("{l}");
            }
            cmd.wait().unwrap();
        }
    }
}
