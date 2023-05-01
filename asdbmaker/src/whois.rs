use std::{fmt::Display, process::Command, string::FromUtf8Error};

#[derive(Debug)]
pub enum Error {
    FailedCommand,
    FailedUTF8Parse,
    FailedSpawnBlocking,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error {self:?}")
    }
}

impl std::error::Error for Error {}
// TODO manually implement Froms from Command or other used libs to use the ? operator
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::FailedCommand
    }
}
impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Error::FailedUTF8Parse
    }
}

// impl From<tokio::task::JoinError> for Error {
//     fn from(e: tokio::task::JoinError) -> Self  {
//         Error::FailedSpawnBlocking
//     }
// }

pub type Result<T> = std::result::Result<T, Error>;

async fn get_asn_details(asn: u32) -> Result<String> {
    // tokio::spawn_blocking(move || {
    let out =
        tokio::task::block_in_place(|| Command::new("whois").arg(format!("AS{asn}")).output())?;

    let stdout = String::from_utf8(out.stdout)?;
    Ok(stdout)
}

async fn get_org_details(org: &str) -> Result<String> {
    let out = tokio::task::block_in_place(|| Command::new("whois").arg(org).output())?;

    let stdout = String::from_utf8(out.stdout)?;
    Ok(stdout)
}

async fn get_org_for_asn(asn: u32) -> String {
    todo!()
}

async fn get_people_for_org(org: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTED_ASN: u32 = 5550;
    const TESTED_ORG: &str = "ORG-TUoG1-RIPE";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_asn_details() {
        let result = get_asn_details(TESTED_ASN).await.unwrap();
        println!("{result}");
        assert!(!result.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_org_details() {
        let result = get_org_details(TESTED_ORG).await.unwrap();
        println!("{result}");
        assert!(!result.is_empty());
    }
}
