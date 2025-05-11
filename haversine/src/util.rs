use core::slice;
use std::{ffi::c_void, fs::File, io::Read, ops::{Index, IndexMut}, os::unix::fs::MetadataExt, ptr::null_mut};

use crate::calc::average_haversine;
use crate::generate::gen_input;
use crate::parse::JsonValue;
use profiler::{clear_profiler, profile_report};
use profiler_macro::instr;

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

pub fn read_to_string_fast(f: &mut File) -> String {
    let mut size_remaining = f.metadata().unwrap().size();

    let mut region = MemRegion::alloc(size_remaining as usize);
    let data = region.as_mut_slice();
    // let mut data = Vec::with_capacity(size_remaining as usize);
    // Causes size remaining to be uninitialized
    // unsafe { data.set_len(size_remaining as usize); }

    let mut pos = 0;

    while size_remaining > 0 {
        let n = f.read(&mut data[pos..]).unwrap();

        size_remaining -= n as u64;
        pos += n;
    }

    // Size remaining is now 0, meaning all of data is initialized after this point

    unsafe { String::from_utf8_unchecked(data.to_vec()) }
}

pub struct MemRegion {
    ptr: *mut u8,
    pub len: usize,
}

impl MemRegion {
    pub fn alloc(len: usize) -> Self {
        let ptr = unsafe {
            match libc::mmap(
                null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) {
                libc::MAP_FAILED => panic!("Failed to map memory"),
                ptr => ptr as *mut u8,
            }
        };

        Self { ptr, len }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.ptr, self.len)
        }
    }
}

impl Drop for MemRegion {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut c_void, self.len);
        }
    }
}

impl Index<usize> for MemRegion {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.ptr.add(index).as_ref().unwrap()
        }
    }
}

impl IndexMut<usize> for MemRegion {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.ptr.add(index).as_mut().unwrap()
        }
    }
}


