# ASDB Builder

This crate provides library for parsing different types of AS data, eg. asrank, whois etc. 
and intserting it into database which can be later used by asmap backend/frontent.

AI description below
---

A library for building and populating an Autonomous System (AS) database from multiple data sources.

## What It Does

`asdb-builder` downloads, processes, and imports AS (Autonomous System) data from three main sources into a MongoDB database:

1. **ASRank** - AS rankings and relationships from CAIDA
2. **IPNetDB** - IP prefix to ASN mappings with geolocation data
3. **Stanford ASDB** - AS classifications and categories

## Architecture

```
asdb-builder (this crate)
    ↓
  asdb (database interface)
    ↓
  MongoDB (storage)
```

## Main Components

### `AsdbBuilder` - Main API

The entry point for all operations:

```rust
let builder = AsdbBuilder::new(
    "mongodb://localhost:27017",
    "asdb_database",
    "inputs"  // where downloads are stored
).await?;

// Clear and prepare database
builder.clear_database().await?;

// Load data from sources
builder.load_asrank_asns(None).await?;      // Downloads from ASRank API
builder.load_ipnetdb().await?;               // Downloads MaxMind DBs
builder.load_stanford_asdb().await?;         // Downloads Stanford classifications
builder.generate_categories().await?;        // Generate AS categories
```

### Data Sources

#### 1. **ASRank** (`src/asrank.rs`)
- **Source**: CAIDA ASRank GraphQL API
- **Data**: AS numbers, names, organizations, country codes, rankings
- **Format**: Downloads as JSONL, imports to MongoDB
- **Key fields**: ASN, name, organization, country, rank

#### 2. **IPNetDB** (`src/ipnetdb.rs`)
- **Source**: IPNetDB MaxMind databases
- **Files**: 
  - `ipnetdb_asn_latest.mmdb` - ASN data
  - `ipnetdb_prefix_latest.mmdb` - Prefix details
- **Data**: IP prefixes, ASN mappings, geolocation
- **Key fields**: IPv4/IPv6 prefixes, locations, ISP info

#### 3. **Stanford ASDB** (`src/stanford_asdb.rs`)
- **Source**: Stanford AS classification datasets
- **Data**: AS classifications (ISP, Content, Enterprise, etc.)
- **Processing**: Generates normalized categories from raw data

## Workflow

```mermaid
graph TD
    A[AsdbBuilder::new] --> B[clear_database]
    B --> C[load_asrank_asns]
    B --> D[load_ipnetdb]
    B --> E[load_stanford_asdb]
    C --> F[Populated MongoDB]
    D --> F
    E --> F
    F --> G[generate_categories]
```

## Key Features

- **Idempotent imports**: Re-running imports won't duplicate data
- **Progress tracking**: Uses `indicatif` progress bars
- **Error handling**: Graceful handling of malformed data
- **Test coverage**: Integration tests with test MongoDB instances

## Database Structure

After import, MongoDB contains:

```
asdb_database/
  ├── asns              # Main AS collection
  │   ├── asn (number)
  │   ├── name
  │   ├── organization
  │   ├── country
  │   ├── asrank_data
  │   ├── ipnetdb_data
  │   └── stanford_data
  └── [other collections...]
```

## Testing

Tests use isolated MongoDB databases (auto-cleaned):

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test import_asrank_asns_fills_asdb
```

Test data is in `test-data/`:
- `asns.jsonl` - Sample ASRank data
- `asns2.jsonl` - Overlapping data for deduplication tests

## Development Setup

1. **Start MongoDB**:
   ```bash
   docker-compose up -d
   ```

2. **Run imports**:
   ```bash
   cargo run -p asmap-cli -- load-asrank
   cargo run -p asmap-cli -- load-ipnetdb
   ```

3. **View data**:
   ```bash
   # Connect to MongoDB
   mongosh mongodb://root:devrootpass@localhost:27017
   ```

## Error Handling

Each data source has its own error type:
- `asrank::Error` - ASRank API/parsing errors
- `ipnetdb::Error` - MaxMind database errors  
- `stanford_asdb::Error` - Stanford data errors

All are wrapped in the top-level `error::Error` enum.

## Dependencies

- **asdb** - Database interface layer
- **maxminddb** - Reading MaxMind .mmdb files
- **trauma** - Parallel file downloads
- **indicatif** - Progress bars
- **mongodb** - Database driver

## Common Issues

**Q: "attempt to subtract with overflow" in ipnetdb**  
A: MaxMind database file is corrupted. Re-download:
```bash
rm -rf inputs/ipnetdb_*.mmdb
cargo run -p asmap-cli -- load-ipnetdb
```

**Q: Tests fail with connection errors**  
A: Ensure MongoDB is running:
```bash
docker-compose up -d
```

## Next Steps for New Developers

1. Read `src/lib.rs` - understand `AsdbBuilder` API
2. Check `asrank.rs` - see how GraphQL data is fetched
3. Look at `ipnetdb.rs` - understand MaxMind processing
4. Review tests in `src/lib.rs` - see integration patterns
5. Explore `asdb` crate - learn database schema

## Contributing

When adding new data sources:
1. Create `src/new_source.rs` module
2. Add `mod new_source;` to `lib.rs`
3. Implement `async fn load(&Asdb) -> Result<()>`
4. Add public method to `AsdbBuilder`
5. Write integration tests

## Questions?

- Check existing tests for usage examples
- Review `asdb` crate for database schema
- Look at CLI usage in `asmap-cli` crate