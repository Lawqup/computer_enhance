use std::{
    fs::File,
    io::{self, stdin, stdout, Read, Write},
    process::Command,
};

use exec::exec;
use parse::{disassemble, Inst, InstStream};

pub mod exec;
pub mod parse;

pub fn assemble(input: &str) -> Vec<u8> {
    let mut tmp_in = tempfile::NamedTempFile::new().unwrap();
    let mut tmp_out = tempfile::NamedTempFile::new().unwrap();

    tmp_in
        .write_all(input.as_bytes())
        .expect("Failed to write to asm file");
    Command::new("nasm")
        .arg(tmp_in.path())
        .arg("-o")
        .arg(tmp_out.path())
        .output()
        .expect("NASM failed to assemble");

    let mut buf = Vec::new();
    tmp_out
        .read_to_end(&mut buf)
        .expect("Failed to read assembled file");

    buf
}

pub fn test_unformatted(test_asm: &str) -> Vec<Inst> {
    println!("TEST ASM:\n\n{test_asm}");
    let expected = assemble(&test_asm);
    let stream = InstStream::from_binary(expected.clone());
    let generated = disassemble(stream.clone());

    println!("GENERATED ASM:\n\n{generated}");
    let actual = assemble(&generated);

    assert_eq!(expected, actual);
    stream.collect()
}

pub fn test_against_string(test_asm: &str) {
    let input = format!("bits 16\n\n{test_asm}");
    test_unformatted(&input);
}

pub fn test_against_file(path: &str) {
    let test_asm = std::fs::read_to_string(path).expect("Failed to read test file");
    test_unformatted(&test_asm);
}

fn main() -> io::Result<()> {
    let mut asm = String::new();
    stdin().read_to_string(&mut asm)?;

    let binary = assemble(&asm);

    let mut execute = false;
    let mut dump = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--exec" => execute = true,
            "--dump" => dump = true,
            _ => (),
        }
    }

    if !execute {
        let stream: Vec<_> = InstStream::from_binary(binary).collect();
        let disas = disassemble(stream.into_iter());

        return stdout().write_all(disas.as_bytes());
    };

    let state = exec(binary);

    if dump {
        let mut outfile = File::create("dump.data")?;
        outfile.write_all(&state.memory)?;
    }

    Ok(())
}
