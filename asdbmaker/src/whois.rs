mod config;

use config::SERVERS_JSON;

use whois_rust::{WhoIs, WhoIsLookupOptions};

async fn get_asn_details(asn: u32) -> String {
    let whois = WhoIs::from_string(SERVERS_JSON).unwrap();
    let result: String = whois
        .lookup(WhoIsLookupOptions::from_string(format!("AS{asn}")).unwrap())
        .unwrap();
    result
}

async fn get_org_details(org: &str) -> String {
    String::new()
}

async fn get_org_for_asn(asn: u32) -> String {
    String::new()
}

async fn get_people_for_org(org: &str) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTED_ASN: u32 = 5550;
    const TESTED_ORG: &str = "";

    #[tokio::test]
    async fn test_get_asn_details() {
        let result = get_asn_details(TESTED_ASN).await;
        println!("{result}");
        assert!(!result.is_empty());
    }
}
