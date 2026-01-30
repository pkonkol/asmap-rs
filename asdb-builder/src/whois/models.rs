//! Data models for WHOIS API responses.

use serde::{Deserialize, Serialize};

/// Root response from RIPE REST API.
#[derive(Debug, Deserialize)]
pub struct RipeResponse {
    pub objects: Option<Objects>,
    #[serde(rename = "errormessages")]
    pub error_messages: Option<ErrorMessages>,
}

#[derive(Debug, Deserialize)]
pub struct Objects {
    pub object: Vec<RipeObject>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorMessages {
    #[serde(rename = "errormessage")]
    pub messages: Vec<ErrorMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorMessage {
    pub severity: String,
    pub text: String,
}

/// Generic RIPE database object.
#[derive(Debug, Deserialize)]
pub struct RipeObject {
    #[serde(rename = "type")]
    pub object_type: String,
    pub attributes: Attributes,
}

#[derive(Debug, Deserialize)]
pub struct Attributes {
    pub attribute: Vec<Attribute>,
}

#[derive(Debug, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    #[serde(rename = "referenced-type")]
    pub referenced_type: Option<String>,
}

/// Parsed Autonomous System information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutNum {
    /// AS number (without "AS" prefix)
    pub asn: u32,
    /// AS name
    pub as_name: Option<String>,
    /// Description lines
    pub descr: Vec<String>,
    /// Organization reference (e.g., "ORG-TUoG1-RIPE")
    pub org: Option<String>,
    /// Admin contact references
    pub admin_c: Vec<String>,
    /// Technical contact references
    pub tech_c: Vec<String>,
    /// Abuse contact reference
    pub abuse_c: Option<String>,
    /// Country code
    pub country: Option<String>,
}

/// Parsed Organisation information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organisation {
    /// Organisation ID (e.g., "ORG-TUoG1-RIPE")
    pub org_id: String,
    /// Organisation name
    pub org_name: String,
    /// Organisation type (e.g., "LIR", "OTHER")
    pub org_type: Option<String>,
    /// Address lines
    pub address: Vec<String>,
    /// Country code
    pub country: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Fax number
    pub fax: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Abuse contact reference
    pub abuse_c: Option<String>,
}

/// Parsed Person/Role information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    /// NIC handle (e.g., "JD1234-RIPE")
    pub nic_hdl: String,
    /// Person or role name
    pub name: String,
    /// Address lines
    pub address: Vec<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Fax number
    pub fax: Option<String>,
    /// Email address
    pub email: Option<String>,
}

/// Combined WHOIS data for an AS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsWhoisData {
    pub aut_num: AutNum,
    pub organisation: Option<Organisation>,
    pub contacts: Vec<Person>,
}

impl RipeObject {
    /// Get the first value for an attribute by name.
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .attribute
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Get all values for an attribute by name.
    pub fn get_attrs(&self, name: &str) -> Vec<&str> {
        self.attributes
            .attribute
            .iter()
            .filter(|a| a.name == name)
            .map(|a| a.value.as_str())
            .collect()
    }
}
