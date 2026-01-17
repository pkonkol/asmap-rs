//! Client-side geocoding module using OpenStreetMap Nominatim API.
//!
//! This module provides geocoding functionality that runs in the browser,
//! calling the Nominatim API directly to convert addresses to coordinates.

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

/// Normalize an address for geocoding.
/// 
/// Applies common transformations to make addresses more geocoding-friendly:
/// - Expands Polish abbreviations (ul., al., pl., st., sw.)
/// - Removes organization prefixes
/// - Normalizes postal codes
/// - Cleans up whitespace and punctuation
fn normalize_address(address: &str) -> String {
    let mut result = address.to_string();
    
    // Remove organization names at the beginning (before street address)
    let org_prefix_re = regex_lite::Regex::new(r"(?i)^[^,]+,\s*(ul\.|ulica|al\.|aleja|pl\.|plac|trakt|st\.)").unwrap();
    result = org_prefix_re.replace(&result, "$1").to_string();
    
    // Normalize Polish street prefixes
    let ul_re = regex_lite::Regex::new(r"(?i)\bul\.\s*").unwrap();
    result = ul_re.replace_all(&result, "ulica ").to_string();
    
    let al_re = regex_lite::Regex::new(r"(?i)\bal\.\s*").unwrap();
    result = al_re.replace_all(&result, "aleja ").to_string();
    
    let pl_re = regex_lite::Regex::new(r"(?i)\bpl\.\s*").unwrap();
    result = pl_re.replace_all(&result, "plac ").to_string();
    
    // Normalize "st." (street) abbreviation
    let st_re = regex_lite::Regex::new(r"(?i)\bst\.\s*").unwrap();
    result = st_re.replace_all(&result, "ulica ").to_string();
    
    // Remove "sw." (świętego/saint) abbreviation dots
    let sw_re = regex_lite::Regex::new(r"(?i)\bsw\.\s*").unwrap();
    result = sw_re.replace_all(&result, "świętego ").to_string();
    
    // Remove "no" or "nr" before house numbers
    let no_re = regex_lite::Regex::new(r"(?i),?\s*\b(no|nr)\.?\s*(\d)").unwrap();
    result = no_re.replace_all(&result, " $2").to_string();
    
    // Normalize Polish postal codes (add dash if missing: 80233 -> 80-233)
    let postal_re = regex_lite::Regex::new(r"\b(\d{2})(\d{3})\b").unwrap();
    result = postal_re.replace_all(&result, "$1-$2").to_string();
    
    // Normalize country names
    let poland_re = regex_lite::Regex::new(r"(?i)\bPOLAND\b").unwrap();
    result = poland_re.replace_all(&result, "Poland").to_string();
    
    let germany_re = regex_lite::Regex::new(r"(?i)\bGERMANY\b").unwrap();
    result = germany_re.replace_all(&result, "Germany").to_string();
    
    // Normalize spaces
    let spaces_re = regex_lite::Regex::new(r"\s+").unwrap();
    result = spaces_re.replace_all(&result, " ").to_string();
    
    // Remove extra commas
    let commas_re = regex_lite::Regex::new(r",\s*,").unwrap();
    result = commas_re.replace_all(&result, ",").to_string();
    
    result.trim().to_string()
}

/// Try to extract just city and country from an address for fallback geocoding.
fn extract_city_country(normalized: &str) -> Option<String> {
    // Try to find postal code pattern followed by city name
    let postal_city_re = regex_lite::Regex::new(r"(\d{2}-\d{3})\s+([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]+)").ok()?;
    if let Some(caps) = postal_city_re.captures(normalized) {
        let city = caps.get(2)?.as_str();
        // Try to find country at the end
        let country_re = regex_lite::Regex::new(r",\s*([A-Za-z]+)\s*$").ok()?;
        if let Some(country_caps) = country_re.captures(normalized) {
            return Some(format!("{}, {}", city, country_caps.get(1)?.as_str()));
        }
        return Some(city.to_string());
    }
    None
}

/// Geocode a single address using Nominatim API.
async fn geocode_single(address: &str) -> Result<(Coordinate, String), String> {
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        urlencoding::encode(address)
    );
    
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
        
        let normalized = normalize_address(&original);
        log!(format!("Geocoding: {} -> {}", original, normalized));
        
        // Try full normalized address first
        match geocode_single(&normalized).await {
            Ok((coord, display_name)) => {
                results.push(GeocodedAddress {
                    original_address: original,
                    normalized_address: normalized,
                    coordinate: Some(coord),
                    display_name: Some(display_name),
                    error: None,
                });
                continue;
            }
            Err(e) if e.contains("No results") => {
                // Try fallback with city/country only
                if let Some(city_country) = extract_city_country(&normalized) {
                    log!(format!("Trying fallback: {}", city_country));
                    // Wait for rate limiting
                    TimeoutFuture::new(1100).await;
                    
                    match geocode_single(&city_country).await {
                        Ok((coord, display_name)) => {
                            results.push(GeocodedAddress {
                                original_address: original,
                                normalized_address: city_country,
                                coordinate: Some(coord),
                                display_name: Some(display_name),
                                error: None,
                            });
                            continue;
                        }
                        Err(fallback_err) => {
                            results.push(GeocodedAddress {
                                original_address: original,
                                normalized_address: normalized,
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
                    normalized_address: normalized,
                    coordinate: None,
                    display_name: None,
                    error: Some(e),
                });
            }
            Err(e) => {
                results.push(GeocodedAddress {
                    original_address: original,
                    normalized_address: normalized,
                    coordinate: None,
                    display_name: None,
                    error: Some(e),
                });
            }
        }
    }
    
    results
}
