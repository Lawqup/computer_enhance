use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    AL,
    CL,
    DL,
    BL,

    AH,
    CH,
    DH,
    BH,

    AX,
    CX,
    DX,
    BX,

    SP,
    BP,
    SI,
    DI,
}

impl Register {
    fn from_encoding(reg: u8, wide: bool) -> Self {
        use Register::*;

        const REG_ENCODING: [Register; 8] = [AL, CL, DL, BL, AH, CH, DH, BH];
        const REG_ENCODING_WIDE: [Register; 8] = [AX, CX, DX, BX, SP, BP, SI, DI];

        if wide {
            REG_ENCODING_WIDE[reg as usize]
        } else {
            REG_ENCODING[reg as usize]
        }
    }

    fn to_string(&self) -> String {
        match self {
            Register::AL => "al",
            Register::CL => "cl",
            Register::DL => "dl",
            Register::BL => "bl",
            Register::AH => "ah",
            Register::CH => "ch",
            Register::DH => "dh",
            Register::BH => "bh",
            Register::AX => "ax",
            Register::CX => "cx",
            Register::DX => "dx",
            Register::BX => "bx",
            Register::SP => "sp",
            Register::BP => "bp",
            Register::SI => "si",
            Register::DI => "di",
        }
        .to_string()
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffAddr {
    pub base: Option<Register>,
    pub index: Option<Register>,
    pub offset: Option<i16>,
}

impl EffAddr {
    fn from_encoding(rm: u8, mode: u8, disp_bytes: &[u8]) -> (usize, Self) {
        use Register::*;

        const EXPRS: [EffAddr; 8] = [
            EffAddr {
                base: Some(BX),
                index: Some(SI),
                offset: None,
            },
            EffAddr {
                base: Some(BX),
                index: Some(DI),
                offset: None,
            },
            EffAddr {
                base: Some(BP),
                index: Some(SI),
                offset: None,
            },
            EffAddr {
                base: Some(BP),
                index: Some(DI),
                offset: None,
            },
            EffAddr {
                base: Some(SI),
                index: None,
                offset: None,
            },
            EffAddr {
                base: Some(DI),
                index: None,
                offset: None,
            },
            EffAddr {
                base: Some(BP),
                index: None,
                offset: None,
            },
            EffAddr {
                base: Some(BX),
                index: None,
                offset: None,
            },
        ];

        let mut base_expr = EXPRS[rm as usize];
        let (size, mut ea) = match mode {
            0b00 => {
                if rm == 0b110 {
                    let (disp_bytes, disp) = get_disp(true, disp_bytes);
                    (
                        disp_bytes,
                        EffAddr {
                            base: None,
                            index: None,
                            offset: Some(disp),
                        },
                    )
                } else {
                    (0, base_expr)
                }
            }
            0b01 => {
                let (disp_bytes, disp) = get_disp(false, disp_bytes);
                base_expr.offset = Some(disp);
                (disp_bytes, base_expr)
            }
            0b10 => {
                let (disp_bytes, disp) = get_disp(true, disp_bytes);
                base_expr.offset = Some(disp);
                (disp_bytes, base_expr)
            }
            _ => panic!("Invalid encoding for effective address expression!"),
        };

        if ea.offset.is_some_and(|disp| disp == 0) {
            ea.offset = None;
        }

        (size, ea)
    }
}

impl Display for EffAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        if let Some(base) = self.base {
            write!(f, "{base}")?;
        }

        if let Some(index) = self.index {
            write!(f, " + {index}")?;
        }

        if let Some(offset) = self.offset {
            if self.base.is_some() {
                if offset < 0 {
                    write!(f, " - {}", offset.abs())?;
                } else {
                    write!(f, " + {offset}")?;
                }
            } else {
                write!(f, "{offset}")?;
            }
        }

        write!(f, "]")?;
        Ok(())
    }
}

const fn get_bit(byte: u8, offset: u8) -> bool {
    get_bits(byte, offset, 1) == 1
}

const fn get_bits(byte: u8, offset: u8, len: u8) -> u8 {
    (byte << offset) >> (8 - len)
}

fn get_data(sign_extend: bool, wide: bool, data_bytes: &[u8]) -> (usize, u16) {
    if !sign_extend && wide {
        (2, u16::from_le_bytes([data_bytes[0], data_bytes[1]]))
    } else {
        (1, data_bytes[0] as u16)
    }
}

fn get_disp(wide: bool, data_bytes: &[u8]) -> (usize, i16) {
    if wide {
        (2, i16::from_le_bytes([data_bytes[0], data_bytes[1]]))
    } else {
        (1, i8::from_le_bytes([data_bytes[0]]) as i16)
    }
}

#[derive(Debug)]
enum ArithOps {
    ADD,
    SUB,
    CMP,
}

impl ArithOps {
    fn from_opcode(byte: u8) -> Option<Self> {
        match byte {
            0b000 => Some(Self::ADD),
            0b101 => Some(Self::SUB),
            0b111 => Some(Self::CMP),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Operand {
    Reg(Register),
    ImmByte(u8),
    ImmWord(u16),
    MemByte(EffAddr),
    MemWord(EffAddr),
    RelOffsetByte(i8),
}

impl Operand {
    fn from_reg_encoding(reg: u8, wide: bool) -> Self {
        Self::Reg(Register::from_encoding(reg, wide))
    }

    fn from_rm_encoding(
        sign_extend: bool,
        wide: bool,
        mode: u8,
        rm: u8,
        disp_bytes: &[u8],
    ) -> (usize, Self) {
        if mode == 0b11 {
            let r2 = Register::from_encoding(rm, wide);
            (0, Self::Reg(r2))
        } else {
            let (disp_size, expr) = EffAddr::from_encoding(rm, mode, disp_bytes);
            (
                disp_size,
                if sign_extend || wide {
                    Self::MemWord(expr)
                } else {
                    Self::MemByte(expr)
                },
            )
        }
    }

    fn from_data_encoding(sign_extend: bool, wide: bool, data_bytes: &[u8]) -> (usize, Self) {
        let (n, data) = get_data(sign_extend, wide, data_bytes);
        (
            n,
            if n == 1 {
                Self::ImmByte(data as u8)
            } else {
                Self::ImmWord(data)
            },
        )
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Reg(x) => write!(f, "{x}"),
            Operand::ImmByte(x) => write!(f, "byte {x}"),
            Operand::ImmWord(x) => write!(f, "word {x}"),
            Operand::MemByte(x) => write!(f, "byte {x}"),
            Operand::MemWord(x) => write!(f, "word {x}"),
            Operand::RelOffsetByte(x) => {
                let offset = x + 2;
                if offset > 0 {
                    write!(f, "$+{offset}+0")
                } else if offset == 0 {
                    write!(f, "$+0")
                } else {
                    write!(f, "${offset}+0")
                }
            }
        }
    }
}

pub enum Inst {
    MOV(Operand, Operand),
    ADD(Operand, Operand),
    SUB(Operand, Operand),
    CMP(Operand, Operand),
    JO(Operand),
    JNO(Operand),
    JB(Operand),
    JNB(Operand),
    JE(Operand),
    JNE(Operand),
    JBE(Operand),
    JNBE(Operand),
    JS(Operand),
    JNS(Operand),
    JP(Operand),
    JNP(Operand),
    JL(Operand),
    JNL(Operand),
    JLE(Operand),
    JNLE(Operand),
    LOOPNZ(Operand),
    LOOPZ(Operand),
    LOOP(Operand),
    JCXZ(Operand),
    HLT,
}

impl Inst {
    pub fn from_encoding(binary: &[u8]) -> Option<(usize, Self)> {
        let byte = binary[0];
        if byte == 0b11110100 {
            Some((1, Self::HLT))
        } else if get_bits(byte, 0, 6) == 0b100010 {
            // Some(Self::MovRmToFromReg)
            let (n, op1, op2) = mod_reg_rm(binary)?;
            Some((n, Self::MOV(op1, op2)))
        } else if get_bits(byte, 0, 4) == 0b1011 {
            // Some(Self::MovImmToReg)
            let wide = get_bit(byte, 4);
            let reg = get_bits(byte, 5, 3);

            let dest = Operand::from_reg_encoding(reg, wide);
            let (data_size, imm) = Operand::from_data_encoding(false, wide, &binary[1..]);

            Some((1 + data_size, Self::MOV(dest, imm)))
        } else if get_bits(byte, 0, 7) == 0b1100011 {
            // Some(Self::MovImmToRm)
            let (n, op1, op2) = imm_to_rm(false, binary)?;
            Some((n, Self::MOV(op1, op2)))
        } else if get_bits(byte, 0, 7) == 0b1010000 {
            // Some(Self::MovMemToAcc)
            let (n, op1, op2) = const_with_acc(false, true, binary)?;
            Some((n, Self::MOV(op1, op2)))
        } else if get_bits(byte, 0, 7) == 0b1010001 {
            // Some(Self::MovAccToMem)
            let (n, op1, op2) = const_with_acc(true, true, binary)?;
            Some((n, Self::MOV(op1, op2)))
        } else if get_bits(byte, 0, 2) == 0b00 && !get_bit(byte, 5) {
            // Some(Self::ArithToFromReg)

            let arith = ArithOps::from_opcode(get_bits(binary[0], 2, 3))
                .expect("Expected arithmetic operation to have a valid arithmetic octal");

            let (n, op1, op2) = mod_reg_rm(binary)?;
            Some((n, Self::new_arithmetic(arith, op1, op2)))
        } else if get_bits(byte, 0, 6) == 0b100000 {
            // Some(Self::ArithImmToRm)

            let arith = ArithOps::from_opcode(get_bits(binary[1], 2, 3))
                .expect("Expected arithmetic operation to have a valid arithmetic octal");

            let (n, op1, op2) = imm_to_rm(true, binary)?;
            Some((n, Self::new_arithmetic(arith, op1, op2)))
        } else if get_bits(byte, 0, 2) == 0b00 && get_bits(byte, 5, 2) == 0b10 {
            // Some(Self::ArithWithAcc)

            let arith = ArithOps::from_opcode(get_bits(binary[0], 2, 3))
                .expect("Expected arithmetic operation to have a valid arithmetic octal");

            let (n, op1, op2) = const_with_acc(false, false, binary)?;
            Some((n, Self::new_arithmetic(arith, op1, op2)))
        } else if get_bits(byte, 0, 4) == 0b0111 {
            // Some(Self::JMP) || Some(Self::LOOP)
            Some(Self::new_jmp(binary))
        } else if get_bits(byte, 0, 6) == 0b111000 {
            Some(Self::new_loop(binary))
        } else {
            None
        }
    }

    fn new_arithmetic(arith: ArithOps, op1: Operand, op2: Operand) -> Self {
        match arith {
            ArithOps::ADD => Self::ADD(op1, op2),
            ArithOps::SUB => Self::SUB(op1, op2),
            ArithOps::CMP => Self::CMP(op1, op2),
        }
    }

    fn new_jmp(binary: &[u8]) -> (usize, Self) {
        let data = Operand::RelOffsetByte(binary[1] as i8);

        let inst = match get_bits(binary[0], 4, 4) {
            0b0000 => Self::JO(data),
            0b0001 => Self::JNO(data),
            0b0010 => Self::JB(data),
            0b0011 => Self::JNB(data),
            0b0100 => Self::JE(data),
            0b0101 => Self::JNE(data),
            0b0110 => Self::JBE(data),
            0b0111 => Self::JNBE(data),
            0b1000 => Self::JS(data),
            0b1001 => Self::JNS(data),
            0b1010 => Self::JP(data),
            0b1011 => Self::JNP(data),
            0b1100 => Self::JL(data),
            0b1101 => Self::JNL(data),
            0b1110 => Self::JLE(data),
            0b1111 => Self::JNLE(data),
            _ => panic!("Match expected 4 bits"),
        };

        (2, inst)
    }

    fn new_loop(binary: &[u8]) -> (usize, Self) {
        let data = Operand::RelOffsetByte(binary[1] as i8);

        let inst = match get_bits(binary[0], 6, 2) {
            0b00 => Self::LOOPNZ(data),
            0b01 => Self::LOOPZ(data),
            0b10 => Self::LOOP(data),
            0b11 => Self::JCXZ(data),
            _ => panic!("Match expected 2 bits"),
        };

        (2, inst)
    }
}

impl Display for Inst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Inst::MOV(op1, op2) => write!(f, "mov {op1}, {op2}"),
            Inst::ADD(op1, op2) => write!(f, "add {op1}, {op2}"),
            Inst::SUB(op1, op2) => write!(f, "sub {op1}, {op2}"),
            Inst::CMP(op1, op2) => write!(f, "cmp {op1}, {op2}"),
            Inst::JO(op1) => write!(f, "jo {op1}"),
            Inst::JNO(op1) => write!(f, "jno {op1}"),
            Inst::JB(op1) => write!(f, "jb {op1}"),
            Inst::JNB(op1) => write!(f, "jnb {op1}"),
            Inst::JE(op1) => write!(f, "je {op1}"),
            Inst::JNE(op1) => write!(f, "jne {op1}"),
            Inst::JBE(op1) => write!(f, "jbe {op1}"),
            Inst::JNBE(op1) => write!(f, "jnbe {op1}"),
            Inst::JS(op1) => write!(f, "js {op1}"),
            Inst::JNS(op1) => write!(f, "jns {op1}"),
            Inst::JP(op1) => write!(f, "jp {op1}"),
            Inst::JNP(op1) => write!(f, "jnp {op1}"),
            Inst::JL(op1) => write!(f, "jl {op1}"),
            Inst::JNL(op1) => write!(f, "jnl {op1}"),
            Inst::JLE(op1) => write!(f, "jle {op1}"),
            Inst::JNLE(op1) => write!(f, "jnle {op1}"),
            Inst::LOOPNZ(op1) => write!(f, "loopnz {op1}"),
            Inst::LOOPZ(op1) => write!(f, "loopz {op1}"),
            Inst::LOOP(op1) => write!(f, "loop {op1}"),
            Inst::JCXZ(op1) => write!(f, "jcxz {op1}"),
            Inst::HLT => write!(f, "hlt"),
        }
    }
}

fn mod_reg_rm(binary: &[u8]) -> Option<(usize, Operand, Operand)> {
    let b1 = binary[0];
    let b2 = binary[1];

    let dest = get_bit(b1, 6);
    let wide = get_bit(b1, 7);

    let mode = get_bits(b2, 0, 2);
    let reg = get_bits(b2, 2, 3);
    let rm = get_bits(b2, 5, 3);

    let mut r1 = Operand::from_reg_encoding(reg, wide);
    let (disp_size, mut r2) = Operand::from_rm_encoding(false, wide, mode, rm, &binary[2..]);

    if !dest {
        let tmp = r1;
        r1 = r2;
        r2 = tmp;
    }

    Some((2 + disp_size, r1, r2))
}

fn imm_to_rm(arith: bool, binary: &[u8]) -> Option<(usize, Operand, Operand)> {
    let b1 = binary[0];
    let b2 = binary[1];

    let sign_extend = if arith { get_bit(b1, 6) } else { false };

    let wide = get_bit(b1, 7);

    let mode = get_bits(b2, 0, 2);
    let rm = get_bits(b2, 5, 3);

    let (disp_size, dest) = Operand::from_rm_encoding(sign_extend, wide, mode, rm, &binary[2..]);

    let (data_size, imm) = Operand::from_data_encoding(sign_extend, wide, &binary[2 + disp_size..]);

    Some((2 + disp_size + data_size, dest, imm))
}

fn const_with_acc(flip: bool, is_mem: bool, binary: &[u8]) -> Option<(usize, Operand, Operand)> {
    let b1 = binary[0];

    let wide = get_bit(b1, 7);
    let (data_size, data) = get_data(false, wide, &binary[1..]);

    let acc = Operand::Reg(if wide { Register::AX } else { Register::AL });
    let constant = if is_mem || flip {
        let addr = EffAddr {
            base: None,
            index: None,
            offset: Some(data as i16),
        };

        if wide {
            Operand::MemWord(addr)
        } else {
            Operand::MemByte(addr)
        }
    } else {
        if wide {
            Operand::ImmWord(data)
        } else {
            Operand::ImmByte(data as u8)
        }
    };

    if flip {
        Some((1 + data_size, constant, acc))
    } else {
        Some((1 + data_size, acc, constant))
    }
}

pub fn disassemble<I>(stream: I) -> String
where
    I: Iterator<Item = Inst>,
{
    let mut disas = String::new();

    disas += "; This file was disassembled by Lawrence\n";
    disas += "bits 16\n\n";

    for inst in stream {
        disas += &inst.to_string();
        disas += "\n";
    }

    disas
}

#[derive(Debug, Clone)]
pub struct InstStream {
    binary: Vec<u8>,
    pub iptr: usize,
}

impl InstStream {
    pub fn from_binary(binary: Vec<u8>) -> Self {
        Self { binary, iptr: 0 }
    }
}

impl Iterator for InstStream {
    type Item = Inst;

    fn next(&mut self) -> Option<Self::Item> {
        while self.iptr < self.binary.len() {
            let Some((n, parsed)) = Inst::from_encoding(&self.binary[self.iptr..]) else {
                return None;
            };

            self.iptr += n;
            return Some(parsed);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_against_file, test_against_string};

    #[test]
    fn mov_reg_to_reg() {
        test_against_string("mov cx, bx");
    }

    #[test]
    fn test_hw1() {
        test_against_file("inputs/listing_0037_single_register_mov.asm");
        test_against_file("inputs/listing_0038_many_register_mov.asm");
    }

    #[test]
    fn mov_8_bit_imm_to_reg() {
        test_against_string("mov cl, 12");
    }

    #[test]
    fn mov_16_bit_imm_to_reg() {
        test_against_string("mov cx, -3922");
    }

    #[test]
    fn mov_src_addr_calc() {
        test_against_string("mov al, [bx + si]");
        test_against_string("mov bx, [bp + di]");
        test_against_string("mov dx, [bp]");
    }

    #[test]
    fn mov_src_addr_calc_d8() {
        test_against_string("mov ah, [bx + si + 4]");
    }

    #[test]
    fn mov_src_addr_calc_d16() {
        test_against_string("mov al, [bx + si + 4999]");
    }

    #[test]
    fn dest_addr_calc() {
        test_against_string("mov [bx + di], cx");
        test_against_string("mov [bp + si], cl");
        test_against_string("mov [bp], ch");
    }

    #[test]
    fn test_hw2() {
        test_against_file("inputs/listing_0039_more_movs.asm");
    }

    #[test]
    fn mov_signed_displacements() {
        test_against_string("mov ax, [bx + di - 37]");
        test_against_string("mov [si - 300], cx");
        test_against_string("mov dx, [bx - 32]");
    }

    #[test]
    fn mov_explicit_sizes() {
        test_against_string("mov [bp + di], byte 7");
        test_against_string("mov [di + 901], word 347");
    }

    #[test]
    fn mov_direct_address() {
        test_against_string("mov bp, [5]");
        test_against_string("mov bx, [3458]");
    }

    #[test]
    fn mov_mem_to_acc() {
        test_against_string("mov ax, [2555]");
        test_against_string("mov ax, [16]");
    }

    #[test]
    fn mov_acc_to_mem() {
        test_against_string("mov [2555], ax");
        test_against_string("mov [16], ax");
    }

    #[test]
    fn test_hw2_challenge() {
        test_against_file("inputs/listing_0040_challenge_movs.asm");
    }

    #[test]
    fn test_add() {
        test_against_string("add bx, [bx+si]");
        test_against_string("add byte [bx], 34");
        test_against_string("add word [bp + si + 1000], 29");
    }

    #[test]
    fn test_cmp() {
        test_against_string("cmp si, 2");
    }

    #[test]
    fn test_hw3() {
        test_against_file("inputs/listing_0041_add_sub_cmp_jnz.asm");
    }
}
