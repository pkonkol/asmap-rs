//! Address normalization for WHOIS data.
//!
//! WHOIS addresses come in many different formats. This module provides
//! tools to normalize them into a format suitable for geocoding APIs.
//!
//! # Supported Address Formats
//!
//! - Polish: "ul. Narutowicza 11/12, 80-233 Gdansk, Poland"
//! - Polish variant: "Trakt sw.Wojciecha 253, 80018, Gdansk, POLAND"
//! - English style: "st. Józef Wassowski, no 12, 80-225, Gdansk, POLAND"
//! - With organization: "Urzad Miasta Sopot, ul. Tadeusza Kosciuszki 25/27, 81-704 Sopot"

use lazy_static::lazy_static;
use regex::Regex;

/// A rule for normalizing addresses.
///
/// Rules are applied in order to transform addresses into a more
/// geocoding-friendly format.
#[derive(Debug, Clone)]
pub struct NormalizationRule {
    /// Name of the rule for debugging/logging.
    pub name: &'static str,
    /// Pattern to match in the address.
    pattern: Regex,
    /// Replacement string (supports regex capture groups).
    replacement: &'static str,
}

impl NormalizationRule {
    /// Create a new normalization rule.
    ///
    /// # Arguments
    /// * `name` - Descriptive name for the rule
    /// * `pattern` - Regex pattern to match
    /// * `replacement` - Replacement string (use $1, $2, etc. for capture groups)
    pub fn new(name: &'static str, pattern: &str, replacement: &'static str) -> Self {
        Self {
            name,
            pattern: Regex::new(pattern).expect("Invalid regex pattern"),
            replacement,
        }
    }

    /// Apply this rule to an address.
    pub fn apply(&self, address: &str) -> String {
        self.pattern.replace_all(address, self.replacement).to_string()
    }
}

/// Address normalizer with configurable rules.
///
/// The normalizer applies a series of rules to transform WHOIS addresses
/// into a format more suitable for geocoding APIs.
#[derive(Debug, Clone)]
pub struct AddressNormalizer {
    rules: Vec<NormalizationRule>,
}

lazy_static! {
    /// Default normalization rules for common WHOIS address formats.
    static ref DEFAULT_RULES: Vec<NormalizationRule> = vec![
        // Remove organization names at the beginning (before street address)
        // Matches patterns like "Company Name, ul." or "Organization, Trakt"
        NormalizationRule::new(
            "remove_org_prefix",
            r"(?i)^[^,]+,\s*(ul\.|ulica|al\.|aleja|pl\.|plac|trakt|st\.)",
            "$1"
        ),
        
        // Normalize Polish street prefixes
        NormalizationRule::new(
            "normalize_ul",
            r"(?i)\bul\.\s*",
            "ulica "
        ),
        NormalizationRule::new(
            "normalize_al",
            r"(?i)\bal\.\s*",
            "aleja "
        ),
        NormalizationRule::new(
            "normalize_pl",
            r"(?i)\bpl\.\s*",
            "plac "
        ),
        
        // Normalize "st." (street) abbreviation
        NormalizationRule::new(
            "normalize_st",
            r"(?i)\bst\.\s*",
            "ulica "
        ),
        
        // Remove "sw." (świętego/saint) abbreviation dots
        NormalizationRule::new(
            "normalize_sw",
            r"(?i)\bsw\.\s*",
            "świętego "
        ),
        
        // Remove "no" or "nr" before house numbers
        NormalizationRule::new(
            "remove_no_prefix",
            r"(?i),?\s*\b(no|nr)\.?\s*(\d)",
            " $2"
        ),
        
        // Normalize Polish postal codes (add space if missing: 80233 -> 80-233)
        NormalizationRule::new(
            "normalize_postal_code_no_dash",
            r"\b(\d{2})(\d{3})\b",
            "$1-$2"
        ),
        
        // Remove extra commas and spaces
        NormalizationRule::new(
            "remove_extra_commas",
            r",\s*,",
            ","
        ),
        NormalizationRule::new(
            "normalize_spaces",
            r"\s+",
            " "
        ),
        
        // Normalize country names to title case
        NormalizationRule::new(
            "normalize_poland",
            r"(?i)\bPOLAND\b",
            "Poland"
        ),
        NormalizationRule::new(
            "normalize_germany",
            r"(?i)\bGERMANY\b",
            "Germany"
        ),
        
        // Trim leading/trailing whitespace and commas
        NormalizationRule::new(
            "trim_leading",
            r"^[\s,]+",
            ""
        ),
        NormalizationRule::new(
            "trim_trailing",
            r"[\s,]+$",
            ""
        ),
    ];
}

impl Default for AddressNormalizer {
    fn default() -> Self {
        Self {
            rules: DEFAULT_RULES.clone(),
        }
    }
}

impl AddressNormalizer {
    /// Create a new address normalizer with custom rules.
    pub fn new(rules: Vec<NormalizationRule>) -> Self {
        Self { rules }
    }

    /// Create an empty normalizer (no rules applied).
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule to the normalizer.
    pub fn add_rule(&mut self, rule: NormalizationRule) {
        self.rules.push(rule);
    }

    /// Add a rule at a specific position.
    pub fn insert_rule(&mut self, index: usize, rule: NormalizationRule) {
        self.rules.insert(index, rule);
    }

    /// Normalize an address by applying all rules in order.
    pub fn normalize(&self, address: &str) -> String {
        let mut result = address.to_string();
        for rule in &self.rules {
            result = rule.apply(&result);
        }
        result.trim().to_string()
    }

    /// Normalize multiple addresses.
    pub fn normalize_all(&self, addresses: &[String]) -> Vec<String> {
        addresses.iter().map(|a| self.normalize(a)).collect()
    }

    /// Extract key components from an address for fallback geocoding.
    ///
    /// Returns a simplified version with just city and country if available.
    pub fn extract_city_country(&self, address: &str) -> Option<String> {
        // First normalize the address
        let normalized = self.normalize(address);
        
        // Try to find postal code pattern followed by city name
        let postal_city_re = Regex::new(r"(\d{2}-\d{3})\s+([A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ]+)").ok()?;
        if let Some(caps) = postal_city_re.captures(&normalized) {
            let city = caps.get(2)?.as_str();
            // Try to find country at the end
            let country_re = Regex::new(r",\s*([A-Za-z]+)\s*$").ok()?;
            if let Some(country_caps) = country_re.captures(&normalized) {
                return Some(format!("{}, {}", city, country_caps.get(1)?.as_str()));
            }
            return Some(city.to_string());
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_polish_ul() {
        let normalizer = AddressNormalizer::default();
        
        let input = "ul. Narutowicza 11/12, 80-233 Gdansk, Poland";
        let result = normalizer.normalize(input);
        
        assert!(result.contains("ulica"));
        assert!(!result.contains("ul."));
        assert!(result.contains("Gdansk"));
        assert!(result.contains("Poland"));
    }

    #[test]
    fn test_normalize_trakt_address() {
        let normalizer = AddressNormalizer::default();
        
        let input = "Trakt sw.Wojciecha 253, 80018, Gdansk, POLAND";
        let result = normalizer.normalize(input);
        
        assert!(result.contains("świętego"));
        assert!(result.contains("Poland")); // Normalized from POLAND
        assert!(result.contains("80-018")); // Postal code normalized
    }

    #[test]
    fn test_normalize_st_with_no() {
        let normalizer = AddressNormalizer::default();
        
        let input = "st. Józef Wassowski, no 12, 80-225, Gdansk, POLAND";
        let result = normalizer.normalize(input);
        
        assert!(result.contains("ulica"));
        assert!(!result.contains("no 12")); // "no" should be removed
        assert!(result.contains("12"));
        assert!(result.contains("Poland"));
    }

    #[test]
    fn test_normalize_with_org_prefix() {
        let normalizer = AddressNormalizer::default();
        
        let input = "Urzad Miasta Sopot, ul. Tadeusza Kosciuszki 25/27, 81-704 Sopot";
        let result = normalizer.normalize(input);
        
        // Organization name should be removed
        assert!(!result.starts_with("Urzad"));
        assert!(result.contains("Kosciuszki"));
        assert!(result.contains("Sopot"));
    }

    #[test]
    fn test_normalize_postal_code_without_dash() {
        let normalizer = AddressNormalizer::default();
        
        let input = "Street 1, 80233 Gdansk, Poland";
        let result = normalizer.normalize(input);
        
        assert!(result.contains("80-233")); // Dash should be added
    }

    #[test]
    fn test_extract_city_country() {
        let normalizer = AddressNormalizer::default();
        
        let input = "ul. Narutowicza 11/12, 80-233 Gdansk, Poland";
        let city_country = normalizer.extract_city_country(input);
        
        assert!(city_country.is_some());
        let result = city_country.unwrap();
        assert!(result.contains("Gdansk"));
    }

    #[test]
    fn test_custom_rule() {
        let mut normalizer = AddressNormalizer::empty();
        normalizer.add_rule(NormalizationRule::new(
            "custom_replace",
            r"(?i)test",
            "replaced"
        ));
        
        let input = "This is a TEST address";
        let result = normalizer.normalize(input);
        
        assert_eq!(result, "This is a replaced address");
    }

    #[test]
    fn test_normalize_all() {
        let normalizer = AddressNormalizer::default();
        
        let addresses = vec![
            "ul. Test 1, 00-001 Warsaw, Poland".to_string(),
            "al. Example 2, 00-002 Krakow, POLAND".to_string(),
        ];
        
        let results = normalizer.normalize_all(&addresses);
        
        assert_eq!(results.len(), 2);
        assert!(results[0].contains("ulica"));
        assert!(results[1].contains("aleja"));
        assert!(results[1].contains("Poland")); // Not POLAND
    }

    #[test]
    fn test_multiple_spaces_normalized() {
        let normalizer = AddressNormalizer::default();
        
        let input = "ul.  Test   Street,   80-001  City";
        let result = normalizer.normalize(input);
        
        assert!(!result.contains("  ")); // No double spaces
    }

    #[test]
    fn test_empty_address() {
        let normalizer = AddressNormalizer::default();
        
        let result = normalizer.normalize("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_address_without_polish_prefixes() {
        let normalizer = AddressNormalizer::default();
        
        let input = "123 Main Street, New York, USA";
        let result = normalizer.normalize(input);
        
        // Should remain mostly unchanged
        assert!(result.contains("123 Main Street"));
        assert!(result.contains("New York"));
    }
}
