use std::io::{self, Read};

use profiler_macro::instr;

use crate::{parse::JsonValue, EARTH_RADIUS};

pub fn average_haversine(path: &str) -> io::Result<(usize, f64)> {

    let mut data;

    instr!("Read" {
        let mut infile = std::fs::File::open(path)?;

        data = String::new(); 
        infile.read_to_string(&mut data)?;
    });

    let json = JsonValue::parse(&data);

    let mut sum = 0.0;
    let pairs = json["pairs"].elements();
    instr!("Sum" {
        for pair in pairs {
            let x0 = &pair["x0"];
            let y0 = &pair["y0"];

            let x1 = &pair["x1"];
            let y1 = &pair["y1"];

            sum += haversine(x0.into(), y0.into(), x1.into(), y1.into());
        }
    });

    Ok((data.len(), sum / pairs.len() as f64))
}

fn haversine(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {

    let d_lat = (y1 - y0).to_radians();
    let d_lon = (x1 - x0).to_radians();
    let lat1 = y0.to_radians();
    let lat2 = y1.to_radians();

    fn square(x: f64) -> f64 {
        x * x
    }

    let a = square((d_lat/2.0).sin()) + lat1.cos() * lat2.cos() * square((d_lon/2.0).sin());

    let c = 2.0 * a.sqrt().asin();

    c * EARTH_RADIUS
}

#[cfg(test)]
mod tests {
    use crate::test_samples;

    #[test]
    fn test_uniform() {
        test_samples(false, 1);
        test_samples(false, 1000);
    }

    #[test]
    fn test_cluster() {
        test_samples(true, 1);
        test_samples(true, 1000);
    }

    #[test]
    fn test_large() {
        test_samples(false, 10_000_000);
        test_samples(true, 10_000_000);
    }
}
