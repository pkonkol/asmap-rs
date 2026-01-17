//! Client-side geocoding module using OpenStreetMap Nominatim API.
//!
//! This module provides geocoding functionality that runs in the browser,
//! calling the Nominatim API directly to convert addresses to coordinates.
//!
//! Nominatim uses free-form search and is designed to handle addresses as-is,
//! including common abbreviations. We do minimal normalization to avoid
//! breaking the search.

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
/// Reorders address parts into format: "postal_code city; street number"
/// This format works better with Nominatim for Polish addresses.
fn normalize_address(address: &str) -> String {
    let mut result = address.to_string();
    
    // Remove organization/institution names at the beginning
    let org_patterns = [
        r"(?i)^[^,]+,\s*(ul\.)",
        r"(?i)^[^,]+,\s*(al\.)",
        r"(?i)^[^,]+,\s*(pl\.)",
        r"(?i)^[^,]+,\s*(trakt)",
        r"(?i)^[^,]+,\s*(\d+\s)",
    ];
    
    for pattern in org_patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern) {
            result = re.replace(&result, "$1").to_string();
        }
    }
    
    // Add space after street abbreviations if missing
    let abbrev_patterns = [
        (r"(?i)\bul\.([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ])", "ul. $1"),
        (r"(?i)\bal\.([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ])", "al. $1"),
        (r"(?i)\bpl\.([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ])", "pl. $1"),
    ];
    
    for (pattern, replacement) in abbrev_patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern) {
            result = re.replace_all(&result, replacement).to_string();
        }
    }
    
    // Try to extract and reorder components: postal_code city, street number
    if let Some(reordered) = reorder_address_parts(&result) {
        return reordered;
    }
    
    // Fallback: just clean up whitespace
    if let Ok(re) = regex_lite::Regex::new(r"\s+") {
        result = re.replace_all(&result, " ").to_string();
    }
    result.trim().to_string()
}

/// Reorder address parts into: "postal_code city; street number"
fn reorder_address_parts(address: &str) -> Option<String> {
    // Extract postal code (XX-XXX or XXXXX format)
    let postal_re = regex_lite::Regex::new(r"(\d{2}-?\d{3})").ok()?;
    let postal_code = postal_re.captures(address)?.get(1)?.as_str();
    
    // Extract city - word after postal code, or before country
    let city = extract_city(address)?;
    
    // Extract street with number (ul./al./pl./Trakt + name + number)
    let street = extract_street(address)?;
    
    // Build reordered address: "postal_code city; street"
    Some(format!("{} {}; {}", postal_code, city, street))
}

/// Extract city name from address
fn extract_city(address: &str) -> Option<String> {
    // Try: postal code followed by city
    let postal_city_re = regex_lite::Regex::new(r"\d{2}-?\d{3}\s+([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]+)").ok()?;
    if let Some(caps) = postal_city_re.captures(address) {
        return Some(caps.get(1)?.as_str().to_string());
    }
    
    // Try: city before country (last two parts)
    let parts: Vec<&str> = address.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 2 {
        // Look for city in second-to-last part (before country)
        let maybe_city = parts[parts.len() - 2];
        // Extract just the city name (might have postal code)
        let city_re = regex_lite::Regex::new(r"([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]{3,})").ok()?;
        if let Some(caps) = city_re.captures(maybe_city) {
            return Some(caps.get(1)?.as_str().to_string());
        }
    }
    
    None
}

/// Extract street name and number from address
fn extract_street(address: &str) -> Option<String> {
    // Match: ul./al./pl. + street name + optional number
    let street_re = regex_lite::Regex::new(
        r"(?i)((?:ul\.|al\.|pl\.|trakt)\s*[A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ\.]+(?:\s+[A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ\.]+)*\s*\d*(?:/\d+)?)"
    ).ok()?;
    
    if let Some(caps) = street_re.captures(address) {
        return Some(caps.get(1)?.as_str().trim().to_string());
    }
    
    // Fallback: try to get first part that looks like a street
    let parts: Vec<&str> = address.split(',').map(|s| s.trim()).collect();
    if !parts.is_empty() {
        let first = parts[0];
        // Check if it starts with street indicator
        if regex_lite::Regex::new(r"(?i)^(ul\.|al\.|pl\.|trakt)").ok()?.is_match(first) {
            return Some(first.to_string());
        }
    }
    
    None
}

/// Try to extract just city and country from an address for fallback geocoding.
fn extract_city_country(address: &str) -> Option<String> {
    // Try to find postal code pattern (XX-XXX or XXXXX) followed by city name
    let postal_city_re = regex_lite::Regex::new(r"(\d{2}-?\d{3})\s+([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]+)").ok()?;
    if let Some(caps) = postal_city_re.captures(address) {
        let city = caps.get(2)?.as_str();
        // Try to find country at the end (last word after comma)
        let country_re = regex_lite::Regex::new(r",\s*([A-Za-z]+)\s*$").ok()?;
        if let Some(country_caps) = country_re.captures(address) {
            return Some(format!("{}, {}", city, country_caps.get(1)?.as_str()));
        }
        return Some(city.to_string());
    }
    
    // Fallback: try to get the last two comma-separated parts (city, country)
    let parts: Vec<&str> = address.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        let last_two = &parts[parts.len()-2..];
        return Some(last_two.join(", "));
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
                log!(format!("No results for: {}", normalized));
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
