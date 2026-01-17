//! Geocoding module for converting WHOIS addresses to coordinates.
//!
//! This module provides functionality to:
//! - Normalize addresses from various WHOIS formats
//! - Geocode addresses to latitude/longitude coordinates using OpenStreetMap Nominatim API
//! - Handle multiple addresses per AS
//!
//! # Example
//!
//! ```ignore
//! use geocoding::{Geocoder, AddressNormalizer};
//!
//! let geocoder = Geocoder::new();
//! let normalizer = AddressNormalizer::default();
//!
//! let addresses = vec![
//!     "ul. Narutowicza 11/12, 80-233 Gdansk, Poland".to_string(),
//!     "Trakt sw.Wojciecha 253, 80018, Gdansk, POLAND".to_string(),
//! ];
//!
//! let coords = geocoder.geocode_addresses(&addresses, &normalizer).await?;
//! ```

pub mod address_normalizer;
pub mod error;
pub mod nominatim;

pub use address_normalizer::{AddressNormalizer, NormalizationRule};
pub use error::{Error, Result};
pub use nominatim::{Coordinate, GeocodedAddress, Geocoder};

/// Convenience function to geocode a list of WHOIS addresses.
///
/// Uses default normalizer rules and Nominatim API.
pub async fn geocode_whois_addresses(addresses: &[String]) -> Result<Vec<GeocodedAddress>> {
    let geocoder = Geocoder::new()?;
    let normalizer = AddressNormalizer::default();
    geocoder.geocode_addresses(addresses, &normalizer).await
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Integration tests that require network access are ignored by default
    // Run with: cargo test -- --ignored
    
    #[tokio::test]
    #[ignore = "requires network access to Nominatim API"]
    async fn test_geocode_real_address() {
        let geocoder = Geocoder::new().unwrap();
        let normalizer = AddressNormalizer::default();
        
        let addresses = vec!["Gdansk, Poland".to_string()];
        let results = geocoder.geocode_addresses(&addresses, &normalizer).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert!(results[0].coordinate.is_some());
        
        let coord = results[0].coordinate.as_ref().unwrap();
        // Gdansk is roughly at 54.35°N, 18.65°E
        assert!(coord.latitude > 54.0 && coord.latitude < 55.0);
        assert!(coord.longitude > 18.0 && coord.longitude < 19.0);
    }

    #[tokio::test]
    #[ignore = "requires network access to Nominatim API"]
    async fn test_geocode_whois_addresses_examples() {
        // Test with actual WHOIS-style addresses from the user's examples
        let addresses = vec![
            "ul. Narutowicza 11/12, 80-233 Gdansk, Poland".to_string(),
            "Trakt sw.Wojciecha 253, 80018, Gdansk, POLAND".to_string(),
            "st. Józef Wassowski, no 12, 80-225, Gdansk, POLAND".to_string(),
            "Urzad Miasta Sopot, ul. Tadeusza Kosciuszki 25/27, 81-704 Sopot".to_string(),
        ];

        let results = geocode_whois_addresses(&addresses).await.unwrap();
        
        assert_eq!(results.len(), 4);
        
        // Print results for manual verification
        for result in &results {
            println!("Original: {}", result.original_address);
            println!("Normalized: {}", result.normalized_address);
            if let Some(coord) = &result.coordinate {
                println!("Coordinates: {:.6}, {:.6}", coord.latitude, coord.longitude);
            }
            if let Some(error) = &result.error {
                println!("Error: {}", error);
            }
            println!("---");
        }
        
        // At least some should succeed (city-level fallback should work)
        let successful = results.iter().filter(|r| r.coordinate.is_some()).count();
        assert!(successful >= 2, "Expected at least 2 successful geocodes, got {}", successful);
    }
}
