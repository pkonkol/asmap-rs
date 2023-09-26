use geocoding::{Forward, Opencage, Openstreetmap, Point};

pub fn georesolve(address: &str) -> Vec<Point<f64>> {
    let osm = Openstreetmap::new();
    let res: Vec<Point<f64>> = osm.forward(address).unwrap();
    res
}

pub fn georesolve2(address: &str) -> Vec<Point<f64>> {
    let api_key = std::env::var("OPENCAGE_KEY").unwrap();
    let oc = Opencage::new(api_key);
    let res: Vec<Point<f64>> = oc.forward(address).unwrap();
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADR1: &str = "ul.Narutowicza 11/12 80-233 Gdansk POLAND";
    const ADR2: &str = "Gdynia";

    #[test]
    fn addresses_osm() {
        for a in [ADR1, ADR2] {
            let cords = georesolve(ADR1);
            println!("osm cords for {a} are {cords:?}");
        }
    }

    #[test]
    fn addresses_oc() {
        for a in [ADR1, ADR2] {
            let cords = georesolve2(ADR1);
            println!("opencage cords for {a} are {cords:?}");
        }
    }
}
