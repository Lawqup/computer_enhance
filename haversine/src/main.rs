use std::{io, ops::Index};

use calc::average_haversine;
use generate::gen_input;
use parse::JsonValue;
use profiler::{profile_report, Profiler, Timer};
use profiler_macro::{instr, instrument};

pub mod generate;
pub mod parse;
pub mod calc;

pub const EARTH_RADIUS: f64 = 6372.8;

impl<'a> Index<usize> for JsonValue<'a> {
    type Output = JsonValue<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        let JsonValue::Array { elements } = self else {
            panic!("can only index with a usize into a json array");
        };

        &elements[index]
    }
}

impl<'a> Index<&str> for JsonValue<'a> {
    type Output = JsonValue<'a>;

    fn index(&self, index: &str) -> &Self::Output {
        let JsonValue::Object { pairs } = self else {
            panic!("Can only index with a string into a JSON object");
        };

        &pairs.iter().find(|(k, _)| *k == index).expect("Key {index} not found").1
    }
}

impl<'a> JsonValue<'a> {
    pub fn elements(&self) -> &Vec<JsonValue<'a>> {
        let JsonValue::Array { elements } = self else {
            panic!("Can only get elements of a json array");
        };

        elements
    }

    pub fn items(&self) -> &Vec<(&str, JsonValue<'a>)> {
        let JsonValue::Object { pairs } = self else {
            panic!("Can only get items of a json array");
        };

        pairs
    }
}

impl<'a> Into<f64> for JsonValue<'a> {
    fn into(self) -> f64 {
        let JsonValue::Number(number) = self else {
            panic!("Tried to get number from {self:?}");
        };

        number
    }
}

impl<'a> Into<&'a str> for JsonValue<'a> {
    fn into(self) -> &'a str {
        let JsonValue::String(s) = self else {
            panic!("Tried to get str from {self:?}");
        };

        s
    }
}

impl<'a> Into<bool> for JsonValue<'a> {
    fn into(self) -> bool {
        let JsonValue::Boolean(b) = self else {
            panic!("Tried to get bool from {self:?}");
        };

        b
    }
}

impl<'a> Into<f64> for &JsonValue<'a> {
    fn into(self) -> f64 {
        let JsonValue::Number(number) = self else {
            panic!("Tried to get number from {self:?}");
        };

        *number
    }
}

impl<'a> Into<&'a str> for &JsonValue<'a> {
    fn into(self) -> &'a str {
        let JsonValue::String(s) = self else {
            panic!("Tried to get str from {self:?}");
        };

        s
    }
}

impl<'a> Into<bool> for &JsonValue<'a> {
    fn into(self) -> bool {
        let JsonValue::Boolean(b) = self else {
            panic!("Tried to get bool from {self:?}");
        };

        *b
    }
}

fn test_samples(uniform: bool, samples: u64) {
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let path = tmpfile.path().to_str().unwrap();

    println!("Generating input -- uniform: {uniform}");
    let expected = gen_input(path, uniform, samples).expect("Failed to generate input");

    println!("Finished gen input");
    let (input_size, actual) = average_haversine(path).expect("Failed to calculate haversine");

    instr!("Output" {
        println!("-------------------------");
        println!("Input size: {input_size}");
        println!("Pair count: {samples}");

        println!("Haversine avg: {actual}\n");

        println!("Validation:");
        println!("Reference avg: {expected}");
        println!("Difference: {}\n", actual - expected);
    });

    profile_report();
    println!("-------------------------\n");

    println!();

    assert_eq!(expected, actual);
}

fn main() -> io::Result<()> {

    test_samples(true, 1_000_000);
    test_samples(false, 1_000_000);
    Ok(())
}
