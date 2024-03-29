use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv6Addr};
use std::process::{Command, Stdio};

use asdb_builder::AsdbBuilder;
use clap::{Args, Parser, Subcommand};

mod details;

const CONFIG_PATH: &str = "config.yaml";

#[derive(Parser)]
#[command(name = "asmap")]
#[command(about = "Cli for managing asmap data and server")]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(short, long, default_value = "inputs/")]
    pub inputs_path: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Resets the database
    ClearDB,
    /// Performs all of the loading steps in the correct order
    LoadAll(LoadAllArgs),
    /// Load asrank asns.jsonl from file, takes in the path
    LoadAsrank(LoadAsrankAsnsArgs),
    /// Downloads if not found and loads IpnetDB data
    LoadIpnetdb,
    /// Downloads and saves the AS categories data from stanford asdb
    LoadStanfordAsdb,
    /// Generates static stanford asdb categories data structure
    GenerateCategories,
    /// Creates detailed file containing information about chosen asns
    GetDetailed(GetDetailedArgs),
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

#[derive(clap::ValueEnum, Clone, Debug)]
enum GetDetailedFormat {
    /// full database dump in json format
    Json,
    /// input list supported by nmap and masscan
    InputList,
}

#[derive(Args)]
struct GetDetailedArgs {
    /// csv file downloaded from asmap website
    #[arg(short, long)]
    pub csv: String,
    /// custom filename for output file. By default derived from input csv path
    #[arg(short, long)]
    pub output: Option<String>,
    /// format of the output file
    #[arg(value_enum, short, long, default_value_t = GetDetailedFormat::InputList)]
    pub format: GetDetailedFormat,
}

#[derive(Args)]
struct StartServerArgs {
    #[arg(short, long, default_value_t = IpAddr::V6(Ipv6Addr::new(0,0,0,0,0,0,0,0)))]
    pub ip: IpAddr,
    #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..), default_value_t = 8080)]
    pub port: u16,
}

#[tokio::main]
async fn main() {
    let cfg = config::parse(CONFIG_PATH);
    let args = Cli::parse();

    std::fs::create_dir_all(&args.inputs_path).expect("Couldn't create input dir");
    match args.command {
        Commands::ClearDB => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.inputs_path)
                .await
                .unwrap();
            m.clear_database().await.unwrap();
        }
        Commands::LoadAll(a) => {
            println!("performing complete database load");
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.inputs_path)
                .await
                .unwrap();
            m.load_asrank_asns(a.asrank_asns_filename).await.unwrap();
            m.load_stanford_asdb().await.unwrap();
            m.load_ipnetdb().await.unwrap();
        }
        Commands::LoadAsrank(a) => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.inputs_path)
                .await
                .unwrap();
            let result = m.load_asrank_asns(a.asns_filename).await;
            println!("import result: {result:?}");
        }
        Commands::LoadIpnetdb => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.inputs_path)
                .await
                .unwrap();
            m.load_ipnetdb().await.unwrap();
        }
        Commands::LoadStanfordAsdb => {
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.inputs_path)
                .await
                .unwrap();
            m.load_stanford_asdb().await.unwrap();
        }
        Commands::GenerateCategories => {
            //This will also be loaded automatically during loadStanfordAsdb but can be also trigerred manually
            let m = AsdbBuilder::new(&cfg.mongo_conn_str, &cfg.db_name, &args.inputs_path)
                .await
                .unwrap();
            m.generate_categories().await.unwrap();
        }
        Commands::GetDetailed(a) => {
            use asdb::Asdb;
            use details::*;
            println!("generating details for {:?}", a.csv);

            let asns = parse_input_csv(&a.csv);

            let asdb = Asdb::new(&cfg.mongo_conn_str, &cfg.db_name).await.unwrap();
            let (ases_detailed, count) = asdb.get_ases(&asns).await.unwrap();
            println!(
                "retrieved {count} detailed ases for {} asns from input",
                asns.len()
            );

            match a.format {
                GetDetailedFormat::InputList => {
                    let output_path = if let Some(p) = a.output {
                        p
                    } else {
                        a.csv.strip_suffix(".csv").unwrap().to_string()
                    };
                    details::generate_nmap_inputlist(&ases_detailed, &output_path);
                }
                GetDetailedFormat::Json => {
                    let output_path = if let Some(p) = a.output {
                        p
                    } else {
                        format!("{}-detailed.json", a.csv.strip_suffix(".csv").unwrap())
                    };
                    generate_json(&ases_detailed, &output_path);
                }
            }
        }
        Commands::Start(_a) => {
            let release_flag = if cfg!(debug_assertions) {
                ""
            } else {
                "--release"
            };
            println!("starting trunk build {release_flag} of frondend app");

            // trunk also needs to build with _a.ip correct
            // TODO, now default works ok tho
            let mut trunk_args = vec![
                "--config",
                "./Trunk.toml",
                "-v",
                "build",
                "--public-url",
                "/",
            ]; //, "index.html", "--public-url", "/", "--dist", "../dist"];
            if !release_flag.is_empty() {
                trunk_args.push(release_flag)
            };
            let mut trunk_cmd = Command::new("trunk")
                .args(trunk_args)
                .current_dir("asmap/frontend/")
                .env("CARGO_TARGET_DIR", "../target-trunk")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            for line in BufReader::new(trunk_cmd.stdout.as_mut().unwrap()).lines() {
                let l = line.unwrap();
                println!("{l}");
            }
            if !trunk_cmd.wait().unwrap().success() {
                panic!("trunk build failed")
            };
            println!("built trunk");

            println!("starting server {release_flag}");
            let (port, addr, config) = (
                _a.port.to_string(),
                _a.ip.to_string(),
                format!("../{CONFIG_PATH}"),
            );
            let mut server_args = vec![
                "run",
                "-p",
                "server",
                "--",
                "--port",
                &port,
                "--addr",
                &addr,
                "--log",
                "debug",
                "--static-dir",
                "./dist",
                "--config",
                &config,
            ];
            if !release_flag.is_empty() {
                server_args.insert(1, release_flag)
            };
            let mut server_cmd = Command::new("cargo")
                .args(server_args)
                .current_dir("asmap/")
                .stdout(Stdio::piped())
                .spawn()
                .expect("Couldn't run the server command");
            for line in BufReader::new(server_cmd.stdout.as_mut().unwrap()).lines() {
                let l = line.unwrap();
                println!("{l}");
            }
            server_cmd.wait().unwrap();
        }
    }
}
