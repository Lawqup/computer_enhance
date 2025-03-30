use std::{io, ops::Index};

use calc::average_haversine;
use generate::gen_input;
use parse::JsonValue;
use profiler::{clear_profiler, profile_report};
use profiler_macro::instr;

pub mod calc;
pub mod generate;
pub mod parse;

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

        &pairs
            .iter()
            .find(|(k, _)| *k == index)
            .expect("Key {index} not found")
            .1
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

impl<'a> From<JsonValue<'a>> for f64 {
    fn from(val: JsonValue<'a>) -> Self {
        let JsonValue::Number(number) = val else {
            panic!("Tried to get number from {val:?}");
        };

        number
    }
}

impl<'a> From<JsonValue<'a>> for &'a str {
    fn from(val: JsonValue<'a>) -> Self {
        let JsonValue::String(s) = val else {
            panic!("Tried to get str from {val:?}");
        };

        s
    }
}

impl<'a> From<JsonValue<'a>> for bool {
    fn from(val: JsonValue<'a>) -> Self {
        let JsonValue::Boolean(b) = val else {
            panic!("Tried to get bool from {val:?}");
        };

        b
    }
}

impl<'a> From<&JsonValue<'a>> for f64 {
    fn from(val: &JsonValue<'a>) -> Self {
        let JsonValue::Number(number) = val else {
            panic!("Tried to get number from {val:?}");
        };

        *number
    }
}

impl<'a> From<&JsonValue<'a>> for &'a str {
    fn from(val: &JsonValue<'a>) -> Self {
        let JsonValue::String(s) = val else {
            panic!("Tried to get str from {val:?}");
        };

        s
    }
}

impl<'a> From<&JsonValue<'a>> for bool {
    fn from(val: &JsonValue<'a>) -> Self {
        let JsonValue::Boolean(b) = val else {
            panic!("Tried to get bool from {val:?}");
        };

        *b
    }
}

fn test_samples(uniform: bool, samples: u64) {
    clear_profiler();
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
    test_samples(false, 1);
    test_samples(false, 1000);
    test_samples(true, 1_000_000);
    test_samples(false, 1_000_000);
    Ok(())
}
