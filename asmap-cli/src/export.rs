use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use futures_util::stream::TryStreamExt;
use mongodb::Client;
use mongodb::bson::Document;
use mongodb::options::{ClientOptions, InsertManyOptions};
use serde::{Deserialize, Serialize};

const COLLECTIONS: [&str; 4] = ["asns", "organisations", "prefixes", "persons"];
const BATCH_SIZE: usize = 1000;

#[derive(Serialize, Deserialize)]
struct ExportRow {
    collection: String,
    doc: Document,
}

async fn connect(conn_str: &str, database: &str) -> mongodb::error::Result<Client> {
    let mut options = ClientOptions::parse(conn_str).await?;
    options.default_database = Some(database.to_string());
    Client::with_options(options)
}

pub async fn export_db(conn_str: &str, database: &str, output_path: &str) -> anyhow::Result<()> {
    let client = connect(conn_str, database).await?;
    let db = client.database(database);

    let file = File::create(output_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut writer = BufWriter::new(encoder);

    for collection_name in COLLECTIONS {
        let collection = db.collection::<Document>(collection_name);
        let mut cursor = collection.find(Document::new()).await?;

        while let Some(doc) = cursor.try_next().await? {
            let row = ExportRow {
                collection: collection_name.to_string(),
                doc,
            };
            let line = serde_json::to_string(&row)?;
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
    }

    writer.flush()?;
    Ok(())
}

pub async fn import_db(conn_str: &str, database: &str, input_path: &str) -> anyhow::Result<()> {
    let client = connect(conn_str, database).await?;
    let db = client.database(database);

    for collection_name in COLLECTIONS {
        let collection = db.collection::<Document>(collection_name);
        let _ = collection.drop().await;
    }

    let file = File::open(input_path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    let mut batch_map: std::collections::HashMap<String, Vec<Document>> =
        std::collections::HashMap::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: ExportRow = serde_json::from_str(&line)?;
        let bucket = batch_map.entry(row.collection.clone()).or_default();
        bucket.push(row.doc);

        if bucket.len() >= BATCH_SIZE {
            let docs = std::mem::take(bucket);
            let collection = db.collection::<Document>(&row.collection);
            collection
                .insert_many(docs)
                .with_options(InsertManyOptions::builder().ordered(false).build())
                .await?;
        }
    }

    for (collection_name, docs) in batch_map {
        if docs.is_empty() {
            continue;
        }
        let collection = db.collection::<Document>(&collection_name);
        collection
            .insert_many(docs)
            .with_options(InsertManyOptions::builder().ordered(false).build())
            .await?;
    }

    Ok(())
}
