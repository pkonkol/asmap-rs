//! OpenStreetMap Nominatim API client for geocoding.
//!
//! This module provides a client for the Nominatim geocoding service,
//! which converts addresses to coordinates.
//!
//! # Rate Limiting
//!
//! Nominatim's usage policy requires:
//! - Maximum 1 request per second
//! - Provide a valid User-Agent identifying your application
//! - Cache results where possible
//!
//! This client automatically enforces rate limiting between requests.

use crate::address_normalizer::AddressNormalizer;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Geographic coordinate (latitude and longitude).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinate {
    /// Create a new coordinate.
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self { latitude, longitude }
    }
}

/// Result of geocoding an address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodedAddress {
    /// Original address string.
    pub original_address: String,
    /// Normalized address used for geocoding.
    pub normalized_address: String,
    /// Resolved coordinate (if successful).
    pub coordinate: Option<Coordinate>,
    /// Display name returned by the geocoder.
    pub display_name: Option<String>,
    /// Error message if geocoding failed.
    pub error: Option<String>,
}

impl GeocodedAddress {
    /// Create a successful geocoded result.
    pub fn success(
        original: String,
        normalized: String,
        coord: Coordinate,
        display_name: String,
    ) -> Self {
        Self {
            original_address: original,
            normalized_address: normalized,
            coordinate: Some(coord),
            display_name: Some(display_name),
            error: None,
        }
    }

    /// Create a failed geocoded result.
    pub fn failure(original: String, normalized: String, error: String) -> Self {
        Self {
            original_address: original,
            normalized_address: normalized,
            coordinate: None,
            display_name: None,
            error: Some(error),
        }
    }
}

/// Response from Nominatim API.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields kept for potential future use
struct NominatimResponse {
    lat: String,
    lon: String,
    display_name: String,
    #[serde(rename = "type")]
    place_type: Option<String>,
    importance: Option<f64>,
}

/// Geocoder client using OpenStreetMap Nominatim API.
pub struct Geocoder {
    client: reqwest::Client,
    base_url: String,
    last_request: Mutex<Option<Instant>>,
    min_request_interval: Duration,
}

impl Geocoder {
    /// Default Nominatim API URL.
    pub const DEFAULT_URL: &'static str = "https://nominatim.openstreetmap.org";

    /// Create a new geocoder with default settings.
    pub fn new() -> Result<Self> {
        Self::with_url(Self::DEFAULT_URL)
    }

    /// Create a new geocoder with a custom Nominatim URL.
    ///
    /// Useful for self-hosted Nominatim instances.
    pub fn with_url(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("asmap-rs/0.1 (https://github.com/pkonkol/asmap-rs)")
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            last_request: Mutex::new(None),
            min_request_interval: Duration::from_millis(1100), // Slightly over 1 second
        })
    }

    /// Create a geocoder for testing with a mock server URL.
    #[cfg(test)]
    pub fn for_testing(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("asmap-rs-test/0.1")
            .timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            last_request: Mutex::new(None),
            min_request_interval: Duration::from_millis(0), // No rate limiting in tests
        })
    }

    /// Enforce rate limiting by waiting if necessary.
    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_request_interval {
                tokio::time::sleep(self.min_request_interval - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }

    /// Geocode a single address.
    ///
    /// Returns the coordinate and display name if found.
    pub async fn geocode(&self, address: &str) -> Result<(Coordinate, String)> {
        self.rate_limit().await;

        let url = format!("{}/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("q", address),
                ("format", "json"),
                ("limit", "1"),
                ("addressdetails", "0"),
            ])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(Error::RateLimitExceeded);
        }

        if !response.status().is_success() {
            return Err(Error::ApiError(format!(
                "HTTP {} from Nominatim",
                response.status()
            )));
        }

        let results: Vec<NominatimResponse> = response.json().await?;

        if results.is_empty() {
            return Err(Error::NoResults(address.to_string()));
        }

        let result = &results[0];
        let lat: f64 = result
            .lat
            .parse()
            .map_err(|_| Error::ApiError("Invalid latitude in response".to_string()))?;
        let lon: f64 = result
            .lon
            .parse()
            .map_err(|_| Error::ApiError("Invalid longitude in response".to_string()))?;

        Ok((Coordinate::new(lat, lon), result.display_name.clone()))
    }

    /// Geocode a single address with fallback to city/country.
    ///
    /// If the full address fails to geocode, tries with just city and country.
    pub async fn geocode_with_fallback(
        &self,
        original: &str,
        normalizer: &AddressNormalizer,
    ) -> GeocodedAddress {
        let normalized = normalizer.normalize(original);

        // Try full normalized address first
        match self.geocode(&normalized).await {
            Ok((coord, display_name)) => {
                return GeocodedAddress::success(
                    original.to_string(),
                    normalized,
                    coord,
                    display_name,
                );
            }
            Err(Error::NoResults(_)) => {
                // Try fallback with city/country only
                if let Some(city_country) = normalizer.extract_city_country(original) {
                    match self.geocode(&city_country).await {
                        Ok((coord, display_name)) => {
                            return GeocodedAddress::success(
                                original.to_string(),
                                city_country,
                                coord,
                                display_name,
                            );
                        }
                        Err(e) => {
                            return GeocodedAddress::failure(
                                original.to_string(),
                                normalized,
                                format!("Fallback geocoding failed: {}", e),
                            );
                        }
                    }
                }
                GeocodedAddress::failure(
                    original.to_string(),
                    normalized,
                    "No results found".to_string(),
                )
            }
            Err(e) => {
                GeocodedAddress::failure(original.to_string(), normalized, e.to_string())
            }
        }
    }

    /// Geocode multiple addresses.
    ///
    /// Handles rate limiting automatically. Returns results in the same order
    /// as the input addresses.
    pub async fn geocode_addresses(
        &self,
        addresses: &[String],
        normalizer: &AddressNormalizer,
    ) -> Result<Vec<GeocodedAddress>> {
        let mut results = Vec::with_capacity(addresses.len());

        for address in addresses {
            let result = self.geocode_with_fallback(address, normalizer).await;
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn mock_nominatim_response(lat: &str, lon: &str, display_name: &str) -> String {
        format!(
            r#"[{{"lat":"{}","lon":"{}","display_name":"{}","type":"city","importance":0.8}}]"#,
            lat, lon, display_name
        )
    }

    #[tokio::test]
    async fn test_geocode_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "Gdansk, Poland"))
            .and(query_param("format", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_nominatim_response(
                "54.3520500",
                "18.6466384",
                "Gdańsk, pomorskie, Polska",
            )))
            .mount(&mock_server)
            .await;

        let geocoder = Geocoder::for_testing(&mock_server.uri()).unwrap();
        let (coord, display_name) = geocoder.geocode("Gdansk, Poland").await.unwrap();

        assert!((coord.latitude - 54.352).abs() < 0.01);
        assert!((coord.longitude - 18.646).abs() < 0.01);
        assert!(display_name.contains("Gdańsk"));
    }

    #[tokio::test]
    async fn test_geocode_no_results() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&mock_server)
            .await;

        let geocoder = Geocoder::for_testing(&mock_server.uri()).unwrap();
        let result = geocoder.geocode("NonexistentPlace12345").await;

        assert!(matches!(result, Err(Error::NoResults(_))));
    }

    #[tokio::test]
    async fn test_geocode_rate_limit_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let geocoder = Geocoder::for_testing(&mock_server.uri()).unwrap();
        let result = geocoder.geocode("Test Address").await;

        assert!(matches!(result, Err(Error::RateLimitExceeded)));
    }

    #[tokio::test]
    async fn test_geocode_with_fallback_full_address_success() {
        let mock_server = MockServer::start().await;

        // The normalizer will transform "ul. Test 1, 80-233 Gdansk, Poland" to contain "ulica"
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_nominatim_response(
                "54.35",
                "18.64",
                "Test Street, Gdańsk",
            )))
            .mount(&mock_server)
            .await;

        let geocoder = Geocoder::for_testing(&mock_server.uri()).unwrap();
        let normalizer = AddressNormalizer::default();

        let result = geocoder
            .geocode_with_fallback("ul. Test 1, 80-233 Gdansk, Poland", &normalizer)
            .await;

        assert!(result.coordinate.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_geocode_addresses_multiple() {
        let mock_server = MockServer::start().await;

        // First address - success
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_nominatim_response(
                "54.35",
                "18.64",
                "Location 1",
            )))
            .expect(2) // Called twice for two addresses
            .mount(&mock_server)
            .await;

        let geocoder = Geocoder::for_testing(&mock_server.uri()).unwrap();
        let normalizer = AddressNormalizer::default();

        let addresses = vec![
            "Address 1, Gdansk, Poland".to_string(),
            "Address 2, Warsaw, Poland".to_string(),
        ];

        let results = geocoder.geocode_addresses(&addresses, &normalizer).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].coordinate.is_some());
        assert!(results[1].coordinate.is_some());
    }

    #[tokio::test]
    async fn test_geocoded_address_success_constructor() {
        let result = GeocodedAddress::success(
            "original".to_string(),
            "normalized".to_string(),
            Coordinate::new(54.35, 18.64),
            "Display Name".to_string(),
        );

        assert_eq!(result.original_address, "original");
        assert_eq!(result.normalized_address, "normalized");
        assert!(result.coordinate.is_some());
        assert!(result.display_name.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_geocoded_address_failure_constructor() {
        let result = GeocodedAddress::failure(
            "original".to_string(),
            "normalized".to_string(),
            "Error message".to_string(),
        );

        assert_eq!(result.original_address, "original");
        assert!(result.coordinate.is_none());
        assert!(result.display_name.is_none());
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Error message");
    }

    #[test]
    fn test_coordinate_new() {
        let coord = Coordinate::new(54.35, 18.64);
        assert_eq!(coord.latitude, 54.35);
        assert_eq!(coord.longitude, 18.64);
    }

    #[test]
    fn test_coordinate_equality() {
        let coord1 = Coordinate::new(54.35, 18.64);
        let coord2 = Coordinate::new(54.35, 18.64);
        let coord3 = Coordinate::new(54.36, 18.64);

        assert_eq!(coord1, coord2);
        assert_ne!(coord1, coord3);
    }
}
