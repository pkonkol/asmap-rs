//! Client-side geocoding module using OpenStreetMap Nominatim API.
//!
//! This module provides geocoding functionality that runs in the browser,
//! calling the Nominatim API directly to convert addresses to coordinates.
//!
//! Uses Nominatim's structured query for better accuracy with Polish addresses.

use gloo_console::log;
use gloo_net::http::Request;
use serde::Deserialize;
use gloo_timers::future::TimeoutFuture;

/// Geographic coordinate.
#[derive(Debug, Clone, Deserialize)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

/// Result of geocoding an address.
#[derive(Debug, Clone)]
pub struct GeocodedAddress {
    pub original_address: String,
    pub normalized_address: String,
    pub coordinate: Option<Coordinate>,
    pub display_name: Option<String>,
    pub error: Option<String>,
}

/// Response from Nominatim API.
#[derive(Debug, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
    display_name: String,
}

/// Parsed address components for structured query.
#[derive(Debug, Clone, Default)]
struct AddressComponents {
    street: Option<String>,
    city: Option<String>,
    postalcode: Option<String>,
    country: Option<String>,
}

impl AddressComponents {
    /// Format as human-readable string for logging
    fn to_display_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref s) = self.street {
            parts.push(format!("street={}", s));
        }
        if let Some(ref c) = self.city {
            parts.push(format!("city={}", c));
        }
        if let Some(ref p) = self.postalcode {
            parts.push(format!("postalcode={}", p));
        }
        if let Some(ref c) = self.country {
            parts.push(format!("country={}", c));
        }
        parts.join(", ")
    }
    
    /// Build URL query string for Nominatim structured search
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref s) = self.street {
            params.push(format!("street={}", urlencoding::encode(s)));
        }
        if let Some(ref c) = self.city {
            params.push(format!("city={}", urlencoding::encode(c)));
        }
        if let Some(ref p) = self.postalcode {
            params.push(format!("postalcode={}", urlencoding::encode(p)));
        }
        if let Some(ref c) = self.country {
            params.push(format!("country={}", urlencoding::encode(c)));
        }
        params.join("&")
    }
    
    /// Check if we have enough components for a meaningful query
    fn is_valid(&self) -> bool {
        self.city.is_some() || self.street.is_some()
    }
    
    /// Create a fallback with just city and country
    fn city_country_fallback(&self) -> Option<AddressComponents> {
        if self.city.is_some() {
            Some(AddressComponents {
                street: None,
                city: self.city.clone(),
                postalcode: None,
                country: self.country.clone(),
            })
        } else {
            None
        }
    }
}

/// Parse address string into structured components.
fn parse_address(address: &str) -> AddressComponents {
    let mut components = AddressComponents::default();
    
    // Clean up the address first - remove org names at the beginning
    let mut cleaned = address.to_string();
    let org_patterns = [
        r"(?i)^[^,]+,\s*(ul\.)",
        r"(?i)^[^,]+,\s*(al\.)",
        r"(?i)^[^,]+,\s*(pl\.)",
        r"(?i)^[^,]+,\s*(trakt)",
    ];
    
    for pattern in org_patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern) {
            cleaned = re.replace(&cleaned, "$1").to_string();
        }
    }
    
    // Add space after street abbreviations if missing: "ul.X" -> "ul. X"
    let abbrev_patterns = [
        (r"(?i)\bul\.([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ])", "ul. $1"),
        (r"(?i)\bal\.([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ])", "al. $1"),
        (r"(?i)\bpl\.([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ])", "pl. $1"),
    ];
    
    for (pattern, replacement) in abbrev_patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern) {
            cleaned = re.replace_all(&cleaned, replacement).to_string();
        }
    }
    
    // Extract postal code (XX-XXX or XXXXX format)
    if let Ok(re) = regex_lite::Regex::new(r"(\d{2}-?\d{3})") {
        if let Some(caps) = re.captures(&cleaned) {
            components.postalcode = Some(caps.get(1).unwrap().as_str().to_string());
        }
    }
    
    // Extract country (last word, typically POLAND/Poland)
    if let Ok(re) = regex_lite::Regex::new(r",\s*([A-Za-z]+)\s*$") {
        if let Some(caps) = re.captures(&cleaned) {
            let country = caps.get(1).unwrap().as_str();
            // Normalize country name
            components.country = Some(country.to_string());
        }
    }
    
    // Extract city - word after postal code
    if let Ok(re) = regex_lite::Regex::new(r"\d{2}-?\d{3}\s+([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]+)") {
        if let Some(caps) = re.captures(&cleaned) {
            components.city = Some(caps.get(1).unwrap().as_str().to_string());
        }
    }
    
    // If no city found after postal code, try second-to-last comma-separated part
    if components.city.is_none() {
        let parts: Vec<&str> = cleaned.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 2 {
            let maybe_city = parts[parts.len() - 2];
            // Extract city name (might have postal code mixed in)
            if let Ok(re) = regex_lite::Regex::new(r"([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]{3,})") {
                if let Some(caps) = re.captures(maybe_city) {
                    components.city = Some(caps.get(1).unwrap().as_str().to_string());
                }
            }
        }
    }
    
    // Extract street with number
    // Match: ul./al./pl./Trakt + street name + number
    if let Ok(re) = regex_lite::Regex::new(
        r"(?i)((?:ul\.|al\.|pl\.|trakt)\s*[A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ\.]+(?:\s+[A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ\.]+)*\s*\d*(?:/\d+)?)"
    ) {
        if let Some(caps) = re.captures(&cleaned) {
            let street = caps.get(1).unwrap().as_str().trim().to_string();
            components.street = Some(street);
        }
    }
    
    // If no street found with prefix, try first comma-separated part
    if components.street.is_none() {
        let parts: Vec<&str> = cleaned.split(',').map(|s| s.trim()).collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            // Check if first part looks like an address (has numbers)
            if let Ok(re) = regex_lite::Regex::new(r"\d") {
                if re.is_match(parts[0]) {
                    components.street = Some(parts[0].to_string());
                }
            }
        }
    }
    
    components
}

/// Geocode using Nominatim structured query.
async fn geocode_structured(components: &AddressComponents) -> Result<(Coordinate, String), String> {
    let query_string = components.to_query_string();
    let url = format!(
        "https://nominatim.openstreetmap.org/search?{}&format=json&limit=1",
        query_string
    );
    
    log!(format!("Nominatim URL: {}", url));
    
    let response = Request::get(&url)
        .header("User-Agent", "asmap-rs/0.1 (https://github.com/pkonkol/asmap-rs)")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    if !response.ok() {
        if response.status() == 429 {
            return Err("Rate limit exceeded".to_string());
        }
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    let results: Vec<NominatimResponse> = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    
    if results.is_empty() {
        return Err("No results found".to_string());
    }
    
    let result = &results[0];
    let lat: f64 = result.lat.parse().map_err(|_| "Invalid latitude")?;
    let lon: f64 = result.lon.parse().map_err(|_| "Invalid longitude")?;
    
    Ok((
        Coordinate { latitude: lat, longitude: lon },
        result.display_name.clone(),
    ))
}

/// Geocode multiple addresses with rate limiting and fallback.
pub async fn geocode_addresses(addresses: Vec<String>) -> Vec<GeocodedAddress> {
    let mut results = Vec::with_capacity(addresses.len());
    
    for (i, original) in addresses.into_iter().enumerate() {
        // Rate limiting: wait 1.1 seconds between requests (Nominatim policy)
        if i > 0 {
            log!(format!("Waiting before next geocode request... ({}/{})", i + 1, results.capacity()));
            TimeoutFuture::new(1100).await;
        }
        
        let components = parse_address(&original);
        let display = components.to_display_string();
        log!(format!("Geocoding: {} -> {}", original, display));
        
        if !components.is_valid() {
            results.push(GeocodedAddress {
                original_address: original,
                normalized_address: display,
                coordinate: None,
                display_name: None,
                error: Some("Could not parse address components".to_string()),
            });
            continue;
        }
        
        // Try structured query with all components
        match geocode_structured(&components).await {
            Ok((coord, display_name)) => {
                results.push(GeocodedAddress {
                    original_address: original,
                    normalized_address: display,
                    coordinate: Some(coord),
                    display_name: Some(display_name),
                    error: None,
                });
                continue;
            }
            Err(e) if e.contains("No results") => {
                // Try fallback with just city/country
                log!(format!("No results for: {}", display));
                if let Some(fallback) = components.city_country_fallback() {
                    let fallback_display = fallback.to_display_string();
                    log!(format!("Trying fallback: {}", fallback_display));
                    // Wait for rate limiting
                    TimeoutFuture::new(1100).await;
                    
                    match geocode_structured(&fallback).await {
                        Ok((coord, display_name)) => {
                            results.push(GeocodedAddress {
                                original_address: original,
                                normalized_address: fallback_display,
                                coordinate: Some(coord),
                                display_name: Some(display_name),
                                error: None,
                            });
                            continue;
                        }
                        Err(fallback_err) => {
                            results.push(GeocodedAddress {
                                original_address: original,
                                normalized_address: display,
                                coordinate: None,
                                display_name: None,
                                error: Some(format!("Fallback failed: {}", fallback_err)),
                            });
                            continue;
                        }
                    }
                }
                results.push(GeocodedAddress {
                    original_address: original,
                    normalized_address: display,
                    coordinate: None,
                    display_name: None,
                    error: Some(e),
                });
            }
            Err(e) => {
                results.push(GeocodedAddress {
                    original_address: original,
                    normalized_address: display,
                    coordinate: None,
                    display_name: None,
                    error: Some(e),
                });
            }
        }
    }
    
    results
}
