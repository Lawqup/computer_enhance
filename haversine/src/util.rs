use crate::calc::average_haversine;
use crate::generate::gen_input;
use crate::parse::JsonValue;
use profiler::{clear_profiler, profile_report};
use profiler_macro::instr;
use std::ops::Index;
use std::usize;
use std::{
    fs::File,
    io::Read,
    os::unix::fs::MetadataExt,
};

#[cfg(feature = "mmap_alloc")]
use crate::allocator::ALLOCATOR;
#[cfg(feature = "mmap_alloc")]
use std::alloc::{GlobalAlloc, Layout};

pub const EARTH_RADIUS: f64 = 6372.8;
pub const KB: usize = 1024;
pub const MB: usize = KB * 1024;
pub const GB: usize = MB * 1024;

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

pub fn test_samples(uniform: bool, samples: u64) {
    clear_profiler();
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let path = tmpfile.path().to_str().unwrap();

    println!("Generating input -- uniform: {uniform}");
    let expected = gen_input(path, uniform, samples).expect("Failed to generate input");

    println!("Finished gen input");
    let (input_size, actual) = average_haversine(path).expect("Failed to calculate haversine");

    instr!("Output", {
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

/// # Safety
///
/// none lmao
#[cfg(feature = "mmap_alloc")]
pub unsafe fn uninit_vec<T>(size: usize) -> Vec<T> {
    let ptr = ALLOCATOR.alloc(Layout::from_size_align_unchecked(size, 1));

    Vec::from_raw_parts(ptr as *mut _, size, size)
}

pub fn read_to_string_fast(f: &mut File) -> String {
    let mut size_remaining = f.metadata().unwrap().size();
    
    #[cfg(feature = "mmap_alloc")]
    let mut data = unsafe { uninit_vec(size_remaining as usize) };

    #[cfg(not(feature = "mmap_alloc"))]
    let mut data = vec![0; size_remaining as usize];

    let mut pos = 0;

    while size_remaining > 0 {
        let n = f.read(&mut data[pos..]).unwrap();

        size_remaining -= n as u64;
        pos += n;
    }

    // Size remaining is now 0, meaning all of data is initialized after this point

    unsafe { String::from_utf8_unchecked(data) }
}
