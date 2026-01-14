//! RIPE NCC REST API client.
//!
//! Provides access to WHOIS data for European, Middle Eastern, and Central Asian networks.
//! API documentation: https://rest.db.ripe.net/

use reqwest::Client;

use super::error::{Error, Result};
use super::models::*;

const RIPE_API_BASE: &str = "https://rest.db.ripe.net";

/// Client for RIPE NCC REST API.
pub struct RipeClient {
    client: Client,
}

impl RipeClient {
    /// Creates a new RIPE API client.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Creates a new RIPE API client with a custom reqwest client.
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Fetches Autonomous System information by ASN.
    ///
    /// # Arguments
    /// * `asn` - AS number (without "AS" prefix)
    ///
    /// # Example
    /// ```ignore
    /// let client = RipeClient::new();
    /// let aut_num = client.get_aut_num(5550).await?;
    /// println!("AS name: {:?}", aut_num.as_name);
    /// ```
    pub async fn get_aut_num(&self, asn: u32) -> Result<AutNum> {
        let url = format!(
            "{}/search.json?query-string=AS{}&type-filter=aut-num&flags=no-filtering",
            RIPE_API_BASE, asn
        );

        let response: RipeResponse = self.client.get(&url).send().await?.json().await?;

        self.check_errors(&response)?;

        let objects = response
            .objects
            .ok_or_else(|| Error::NotFound(format!("AS{asn}")))?;

        let obj = objects
            .object
            .into_iter()
            .find(|o| o.object_type == "aut-num")
            .ok_or_else(|| Error::NotFound(format!("AS{asn}")))?;

        Ok(self.parse_aut_num(&obj, asn))
    }

    /// Fetches Organisation information by org ID.
    ///
    /// # Arguments
    /// * `org_id` - Organisation ID (e.g., "ORG-TUoG1-RIPE")
    pub async fn get_organisation(&self, org_id: &str) -> Result<Organisation> {
        let url = format!(
            "{}/search.json?query-string={}&type-filter=organisation&flags=no-filtering",
            RIPE_API_BASE, org_id
        );

        let response: RipeResponse = self.client.get(&url).send().await?.json().await?;

        self.check_errors(&response)?;

        let objects = response
            .objects
            .ok_or_else(|| Error::NotFound(org_id.to_string()))?;

        let obj = objects
            .object
            .into_iter()
            .find(|o| o.object_type == "organisation")
            .ok_or_else(|| Error::NotFound(org_id.to_string()))?;

        Ok(self.parse_organisation(&obj, org_id))
    }

    /// Fetches Person or Role information by NIC handle.
    ///
    /// # Arguments
    /// * `nic_hdl` - NIC handle (e.g., "JD1234-RIPE")
    pub async fn get_person(&self, nic_hdl: &str) -> Result<Person> {
        let url = format!(
            "{}/search.json?query-string={}&type-filter=person,role&flags=no-filtering",
            RIPE_API_BASE, nic_hdl
        );

        let response: RipeResponse = self.client.get(&url).send().await?.json().await?;

        self.check_errors(&response)?;

        let objects = response
            .objects
            .ok_or_else(|| Error::NotFound(nic_hdl.to_string()))?;

        let obj = objects
            .object
            .into_iter()
            .find(|o| o.object_type == "person" || o.object_type == "role")
            .ok_or_else(|| Error::NotFound(nic_hdl.to_string()))?;

        Ok(self.parse_person(&obj, nic_hdl))
    }

    /// Fetches complete WHOIS data for an AS including organisation and contacts.
    ///
    /// # Arguments
    /// * `asn` - AS number (without "AS" prefix)
    pub async fn get_as_whois_data(&self, asn: u32) -> Result<AsWhoisData> {
        let aut_num = self.get_aut_num(asn).await?;

        // Fetch organisation if referenced
        let organisation = if let Some(ref org_id) = aut_num.org {
            self.get_organisation(org_id).await.ok()
        } else {
            None
        };

        // Collect unique contact references
        let mut contact_refs: Vec<&str> = Vec::new();
        contact_refs.extend(aut_num.admin_c.iter().map(|s| s.as_str()));
        contact_refs.extend(aut_num.tech_c.iter().map(|s| s.as_str()));
        if let Some(ref abuse) = aut_num.abuse_c {
            contact_refs.push(abuse);
        }
        contact_refs.sort();
        contact_refs.dedup();

        // Fetch contact details (ignore errors for individual contacts)
        let mut contacts = Vec::new();
        for nic_hdl in contact_refs {
            if let Ok(person) = self.get_person(nic_hdl).await {
                contacts.push(person);
            }
        }

        Ok(AsWhoisData {
            aut_num,
            organisation,
            contacts,
        })
    }

    fn check_errors(&self, response: &RipeResponse) -> Result<()> {
        if let Some(ref errors) = response.error_messages {
            for msg in &errors.messages {
                if msg.text.contains("ERROR:101") || msg.text.contains("no entries found") {
                    return Err(Error::NotFound(msg.text.clone()));
                }
            }
        }
        Ok(())
    }

    fn parse_aut_num(&self, obj: &RipeObject, asn: u32) -> AutNum {
        AutNum {
            asn,
            as_name: obj.get_attr("as-name").map(String::from),
            descr: obj.get_attrs("descr").into_iter().map(String::from).collect(),
            org: obj.get_attr("org").map(String::from),
            admin_c: obj.get_attrs("admin-c").into_iter().map(String::from).collect(),
            tech_c: obj.get_attrs("tech-c").into_iter().map(String::from).collect(),
            abuse_c: obj.get_attr("abuse-c").map(String::from),
            country: obj.get_attr("country").map(String::from),
        }
    }

    fn parse_organisation(&self, obj: &RipeObject, org_id: &str) -> Organisation {
        Organisation {
            org_id: org_id.to_string(),
            org_name: obj.get_attr("org-name").unwrap_or("").to_string(),
            org_type: obj.get_attr("org-type").map(String::from),
            address: obj.get_attrs("address").into_iter().map(String::from).collect(),
            country: obj.get_attr("country").map(String::from),
            phone: obj.get_attr("phone").map(String::from),
            fax: obj.get_attr("fax-no").map(String::from),
            email: obj.get_attr("e-mail").map(String::from),
            abuse_c: obj.get_attr("abuse-c").map(String::from),
        }
    }

    fn parse_person(&self, obj: &RipeObject, nic_hdl: &str) -> Person {
        let name = obj
            .get_attr("person")
            .or_else(|| obj.get_attr("role"))
            .unwrap_or("")
            .to_string();

        Person {
            nic_hdl: nic_hdl.to_string(),
            name,
            address: obj.get_attrs("address").into_iter().map(String::from).collect(),
            phone: obj.get_attr("phone").map(String::from),
            fax: obj.get_attr("fax-no").map(String::from),
            email: obj.get_attr("e-mail").map(String::from),
        }
    }
}

impl Default for RipeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ASN: u32 = 5550;
    const TEST_ORG: &str = "ORG-TUoG1-RIPE";

    #[tokio::test]
    async fn test_get_aut_num() {
        let client = RipeClient::new();
        let result = client.get_aut_num(TEST_ASN).await;

        assert!(result.is_ok(), "Failed to get AS: {:?}", result.err());
        let aut_num = result.unwrap();
        assert_eq!(aut_num.asn, TEST_ASN);
        assert!(aut_num.as_name.is_some());
        println!("AS{}: {:?}", aut_num.asn, aut_num.as_name);
        println!("Org: {:?}", aut_num.org);
    }

    #[tokio::test]
    async fn test_get_organisation() {
        let client = RipeClient::new();
        let result = client.get_organisation(TEST_ORG).await;

        assert!(result.is_ok(), "Failed to get org: {:?}", result.err());
        let org = result.unwrap();
        assert_eq!(org.org_id, TEST_ORG);
        println!("Org: {} - {}", org.org_id, org.org_name);
        println!("Address: {:?}", org.address);
    }

    #[tokio::test]
    async fn test_get_as_whois_data() {
        let client = RipeClient::new();
        let result = client.get_as_whois_data(TEST_ASN).await;

        assert!(result.is_ok(), "Failed to get WHOIS data: {:?}", result.err());
        let data = result.unwrap();
        println!("AS{}: {:?}", data.aut_num.asn, data.aut_num.as_name);
        if let Some(ref org) = data.organisation {
            println!("Organisation: {}", org.org_name);
        }
        println!("Contacts: {}", data.contacts.len());
        for contact in &data.contacts {
            println!("  - {}: {}", contact.nic_hdl, contact.name);
        }
    }

    #[tokio::test]
    async fn test_not_found() {
        let client = RipeClient::new();
        let result = client.get_aut_num(999999999).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NotFound(_)));
    }
}
