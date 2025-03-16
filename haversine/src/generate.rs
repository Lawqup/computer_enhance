use std::io::{self, BufWriter};
use io::Write;

const X_LB: f64 = -180.0;
const X_UB: f64 = 180.0;

const Y_LB: f64 = -90.0;
const Y_UB: f64 = 90.0;

use profiler::Timer;
use rand::Rng;

use crate::EARTH_RADIUS;

pub fn gen_input(outpath: &str, uniform: bool, samples: u64) -> io::Result<f64> {
    println!("Generating input -- uniform: {uniform}");
    let mut gen = Timer::new("Gen input");
    gen.start();

    let outfile = std::fs::File::create(outpath)?;
    let mut writer = BufWriter::new(outfile);

    let mut rng = rand::rng();

    writeln!(&mut writer, "{{")?;
    writeln!(&mut writer, "    \"pairs\": [")?;

    let mut xa;
    let mut xb;
    let mut ya;
    let mut yb;

    if uniform {
        xa = X_LB;
        xb = X_UB;

        ya = Y_LB;
        yb = Y_UB;

    } else {
        xa = rng.random_range(X_LB..X_UB);
        xb = rng.random_range(X_LB..X_UB);

        if xa > xb {
            (xa, xb) = (xb, xa)
        }

        ya = rng.random_range(Y_LB..Y_UB);
        yb = rng.random_range(Y_LB..Y_UB);

        if ya > yb {
            (ya, yb) = (yb, ya)
        }
    }

    let mut sum = 0.0;
    for sample in 0..samples {
        let x0 = rng.random_range(xa..xb);
        let x1 = rng.random_range(xa..xb);


        let y0 = rng.random_range(ya..yb);
        let y1 = rng.random_range(ya..yb);

        write!(writer, "      {{\"x0\": {x0}, \"y0\": {y0}, \"x1\": {x1}, \"y1\": {y1}}}")?;

        if sample < samples - 1 {
            writeln!(writer, ",")?;
        } else {
            writeln!(writer)?;
        }

        sum += reference_haversine(x0, y0, x1, y1);
    }

    writeln!(&mut writer, "    ]")?;
    writeln!(&mut writer, "}}")?;
    
    gen.stop();
    gen.report_standalone();

    Ok(sum / samples as f64)
}


fn reference_haversine(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {

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
