use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr};
use std::process::{Command, Stdio};

use asdbmaker::Asdbmaker;
use clap::{Args, Parser, Subcommand};

const INPUTS_PATH: &str = "asdbmaker/inputs";
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
    /// Load asrank asns.jsonl from file, takes in the path
    /// todo default path value to inputs
    LoadAsrankAsns(LoadAsrankAsnsArgs),
    /// Starts a server with the map
    StartServer(StartServerArgs),
    // Todo LoadWhois (for range?), LoadIpnetDB, Georesolve(Persons|Orgs|Somethin else?)
}

#[derive(Args)]
struct LoadAsrankAsnsArgs {
    #[arg(short, long, default_value = ASNS_JSONL)]
    pub asns_filename: String,
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
        Commands::LoadAsrankAsns(a) => {
            let m = Asdbmaker::new(&cfg.mongo_conn_str, &cfg.db_name, &args.iputs_path)
                .await
                .unwrap();
            let jsonl_path = a.asns_filename;
            println!("starting import");
            let result = m.import_asrank_asns(&jsonl_path).await;
            println!("import result: {result:?}");
        }
        Commands::StartServer(a) => {
            // TODO pass ip and port from `a`
            let mut cmd = Command::new(&SERVER_DEV_SCRIPT)
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
