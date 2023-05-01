mod asrank;
mod ipnetdb;
mod whois;

// TODO decide API
pub fn create_initial_database() -> () {
    //
}

pub fn download_asn_details(asn: u32) -> () {
    //
}

pub fn download_org_details(asn: u32) -> () {
    // ripe only at first,
    // gets orga
    //
}

//pub fn download_org_details_from_org(org: &str) -> () { }

pub fn download_prefixes(asn: u32) -> () {
    //
}

pub fn download_peers(asn: u32) -> () {
    //
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_smth() {
        assert!(true);
    }
}
