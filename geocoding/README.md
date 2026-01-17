use geocoding::{geocode_whois_addresses, AddressNormalizer, Geocoder};

// Simple usage
let addresses = vec![
    "ul. Narutowicza 11/12, 80-233 Gdansk, Poland".to_string(),
];
let results = geocode_whois_addresses(&addresses).await?;

// Custom usage with normalizer
let geocoder = Geocoder::new()?;
let normalizer = AddressNormalizer::default();
let results = geocoder.geocode_addresses(&addresses, &normalizer).await?;
