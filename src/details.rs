use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use asdb_models::As;

/// generates full dump of database objects to given path for each asn from input
pub fn generate_json(ases: &[As], path: &impl AsRef<Path>) {
    println!("generating detailed json");
    // let ases_json = serde_json::to_string(ases).unwrap();
    let file = File::create(path).unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, ases).unwrap();
    writer.flush().unwrap();
}

/// prefixes with comments for use in `nmap -iL <path>`, also works with `masscan -iL <path> -p 80`
/// and `rustscan -a <path>`
pub fn generate_nmap_inputlist<T: Into<PathBuf> + AsRef<OsStr>>(ases: &[As], base_path: &T) {
    println!("generating inputlist");
    let mut pb_v4 = PathBuf::from(base_path);
    let mut pb_v6 = PathBuf::from(base_path);
    let filename = pb_v4.file_name().unwrap().to_os_string();
    pb_v4.set_file_name(format!("{}-prefixes-v4.txt", filename.to_str().unwrap()));
    pb_v6.set_file_name(format!("{}-prefixes-v6.txt", filename.to_str().unwrap()));
    let file_v4 = File::create(pb_v4).unwrap();
    let file_v6 = File::create(pb_v6).unwrap();
    let mut writer_v4 = BufWriter::new(file_v4);
    let mut writer_v6 = BufWriter::new(file_v6);
    for as_ in ases {
        if as_.ipnetdb_data.is_none() {
            continue;
        }
        let v4_prefixes = &as_.ipnetdb_data.as_ref().unwrap().ipv4_prefixes;
        write!(&mut writer_v4, "# AS{}\n", as_.asn).unwrap();
        for p in v4_prefixes {
            write!(&mut writer_v4, "{}\n", p.range).unwrap();
        }
        let v6_prefixes = &as_.ipnetdb_data.as_ref().unwrap().ipv6_prefixes;
        write!(&mut writer_v6, "# AS{}\n", as_.asn).unwrap();
        for p in v6_prefixes {
            write!(&mut writer_v6, "{}\n", p.range).unwrap();
        }
    }
}

/// returns vec of asns contained in input csv
pub fn parse_input_csv(path: &impl AsRef<Path>) -> Vec<u32> {
    println!("parse input csv");
    let mut out = vec![];
    // let bar = indicatif::ProgressBar::new(BufReader::new(File::open(csv)?).lines().count() as u64);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .unwrap();
    for result in rdr.records() {
        let l = result.unwrap();
        // println!("read line: {l:?}");
        // l.len();
        out.push(l.get(0).unwrap().parse().unwrap());
    }
    out
}

// AsForDetailsJson model?
