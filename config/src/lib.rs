use config::Config;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct MyConfig {
    pub mongo_conn_str: String,
    pub db_name: String,
}

pub fn parse(path: &str) -> MyConfig {
    let cfg: MyConfig = Config::builder()
        .add_source(config::File::with_name(path))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    cfg
}
