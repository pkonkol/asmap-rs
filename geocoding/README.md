# Geocoding Module

Address geocoding library for converting WHOIS addresses to geographic coordinates (latitude/longitude).

## Features

- 🌍 **OpenStreetMap Nominatim API** integration for geocoding
- 📝 **Smart address normalization** for various WHOIS formats
- 🇵🇱 **Polish address support** with abbreviations (ul., al., pl., sw., etc.)
- 🔄 **Fallback geocoding** - falls back to city/country if full address fails
- ⏱️ **Automatic rate limiting** - respects Nominatim's 1 req/sec policy
- 📦 **Multiple addresses** - handle lists of addresses per AS
- 🔧 **Extensible rules** - easily add custom normalization patterns
- ✅ **Well tested** - 20+ unit tests with mocked API responses

## Supported Address Formats

The normalizer handles various WHOIS address formats:

```
ul. Narutowicza 11/12, 80-233 Gdansk, Poland
Trakt sw.Wojciecha 253, 80018, Gdansk, POLAND
st. Józef Wassowski, no 12, 80-225, Gdansk, POLAND
Urzad Miasta Sopot, ul. Tadeusza Kosciuszki 25/27, 81-704 Sopot
123 Main Street, New York, USA
```

## Usage

### Basic Usage

```rust
use geocoding::geocode_whois_addresses;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addresses = vec![
        "ul. Narutowicza 11/12, 80-233 Gdansk, Poland".to_string(),
        "Trakt sw.Wojciecha 253, 80018, Gdansk, POLAND".to_string(),
    ];
    
    let results = geocode_whois_addresses(&addresses).await?;
    
    for result in results {
        println!("Original: {}", result.original_address);
        println!("Normalized: {}", result.normalized_address);
        
        if let Some(coord) = result.coordinate {
            println!("📍 Coordinates: {:.6}, {:.6}", 
                coord.latitude, coord.longitude);
        }
        
        if let Some(error) = result.error {
            println!("❌ Error: {}", error);
        }
        println!();
    }
    
    Ok(())
}
```

### Custom Geocoder Configuration

```rust
use geocoding::{Geocoder, AddressNormalizer, NormalizationRule};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create geocoder
    let geocoder = Geocoder::new()?;
    
    // Use default normalizer with built-in rules
    let mut normalizer = AddressNormalizer::default();
    
    // Add custom normalization rule
    normalizer.add_rule(NormalizationRule::new(
        "custom_abbreviation",
        r"(?i)\bstr\.\s*",
        "street "
    ));
    
    // Geocode addresses
    let addresses = vec!["str. Example 123, City, Country".to_string()];
    let results = geocoder.geocode_addresses(&addresses, &normalizer).await?;
    
    Ok(())
}
```

### Using a Self-Hosted Nominatim Instance

```rust
use geocoding::{Geocoder, AddressNormalizer};

let geocoder = Geocoder::with_url("http://localhost:8080")?;
let normalizer = AddressNormalizer::default();

let results = geocoder.geocode_addresses(&addresses, &normalizer).await?;
```

## Address Normalization

The `AddressNormalizer` applies a series of rules to transform addresses:

### Default Normalization Rules

1. **Organization prefix removal** - Removes company names before street address
2. **Polish street prefixes** - Normalizes `ul.` → `ulica`, `al.` → `aleja`, etc.
3. **Street abbreviations** - Converts `st.` to `ulica`
4. **Saint abbreviations** - Expands `sw.` to `świętego`
5. **Number prefixes** - Removes `no` or `nr` before house numbers
6. **Postal codes** - Adds dashes to Polish postal codes (80233 → 80-233)
7. **Country normalization** - Converts `POLAND` to `Poland`
8. **Whitespace cleanup** - Normalizes multiple spaces and commas

### Adding Custom Rules

```rust
use geocoding::{AddressNormalizer, NormalizationRule};

let mut normalizer = AddressNormalizer::empty();

// Add custom rule with regex pattern and replacement
normalizer.add_rule(NormalizationRule::new(
    "remove_apartment",
    r"(?i),?\s*apt\.?\s*\d+",
    ""
));

// Insert rule at specific position
normalizer.insert_rule(0, NormalizationRule::new(
    "first_rule",
    r"pattern",
    "replacement"
));

let normalized = normalizer.normalize("Address apt. 5, City");
```

## API Response Structure

```rust
pub struct GeocodedAddress {
    /// Original address string from WHOIS
    pub original_address: String,
    
    /// Normalized address used for geocoding
    pub normalized_address: String,
    
    /// Resolved coordinate (if successful)
    pub coordinate: Option<Coordinate>,
    
    /// Display name returned by the geocoder
    pub display_name: Option<String>,
    
    /// Error message if geocoding failed
    pub error: Option<String>,
}

pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}
```

## Rate Limiting

The geocoder automatically enforces Nominatim's usage policy:
- Maximum **1 request per second**
- Requests are automatically queued and delayed as needed
- No manual rate limit management required

## Testing

```bash
# Run unit tests (with mocked API)
cargo test -p geocoding@0.1.0

# Run all tests including integration tests with real API calls
cargo test -p geocoding@0.1.0 -- --include-ignored

# Run specific test
cargo test -p geocoding@0.1.0 test_normalize_polish_ul
```

## Error Handling

```rust
use geocoding::{Error, Geocoder, AddressNormalizer};

match geocoder.geocode("Address").await {
    Ok((coord, display_name)) => {
        println!("Found: {} at {}, {}", display_name, coord.latitude, coord.longitude);
    }
    Err(Error::NoResults(address)) => {
        eprintln!("No results found for: {}", address);
    }
    Err(Error::RateLimitExceeded) => {
        eprintln!("Rate limit exceeded, wait before retrying");
    }
    Err(Error::HttpRequest(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(e) => {
        eprintln!("Geocoding error: {}", e);
    }
}
```

## Dependencies

- `reqwest` - HTTP client for API calls
- `serde` / `serde_json` - JSON serialization
- `regex` / `lazy_static` - Address pattern matching
- `tokio` - Async runtime
- `thiserror` - Error handling
- `wiremock` - HTTP mocking for tests (dev dependency)

## Nominatim Usage Policy

When using the public Nominatim API, please be aware of their [Usage Policy](https://operations.osmfoundation.org/policies/nominatim/):

- ✅ Maximum 1 request per second (enforced by this library)
- ✅ Include a valid User-Agent (automatically set by this library)
- ✅ Cache results where possible
- ✅ No heavy uses (bulk geocoding of entire databases)

For heavy usage, consider:
- Setting up your own [Nominatim instance](https://nominatim.org/release-docs/latest/admin/Installation/)
- Using a commercial geocoding service
- Caching geocoded results in your database

## License

MIT
