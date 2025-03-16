use std::fmt::Display;

use crate::{
    assemble,
    parse::{EffAddr, Inst, Operand, Register},
};

const REGISTER_SIZE: usize = 8 * 2;
const MEM_SIZE: usize = 1 << 16;

struct GeneralRegisters {
    reg_array: Box<[u8; REGISTER_SIZE]>,
}

impl GeneralRegisters {
    pub fn new() -> Self {
        Self {
            reg_array: Box::new([0; REGISTER_SIZE]),
        }
    }

    fn reg_pos(reg: Register) -> (usize, bool) {
        match reg {
            Register::AL => (0, false),
            Register::CL => (2, false),
            Register::DL => (4, false),
            Register::BL => (6, false),

            Register::AH => (1, false),
            Register::CH => (3, false),
            Register::DH => (5, false),
            Register::BH => (7, false),

            Register::AX => (0, true),
            Register::CX => (2, true),
            Register::DX => (4, true),
            Register::BX => (6, true),

            Register::SP => (8, true),
            Register::BP => (10, true),
            Register::SI => (12, true),
            Register::DI => (14, true),
        }
    }

    pub fn get_reg(&self, reg: Register) -> u16 {
        let (pos, wide) = Self::reg_pos(reg);

        if wide {
            u16::from_le_bytes([self.reg_array[pos], self.reg_array[pos + 1]])
        } else {
            self.reg_array[pos] as u16
        }
    }

    pub fn set_reg(&mut self, reg: Register, val: u16) {
        let (pos, wide) = Self::reg_pos(reg);

        let before = self.get_reg(reg);

        if wide {
            let bytes = val.to_le_bytes();
            self.reg_array[pos] = bytes[0];
            self.reg_array[pos + 1] = bytes[1];
        } else {
            self.reg_array[pos] = val as u8;
        };

        print!(" {reg}:0x{before:x}->0x{:x}", self.get_reg(reg))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Parity = 1 << 2,
    Zero = 1 << 6,
    Signed = 1 << 7,
}

impl Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_str = match self {
            Flag::Parity => "P",
            Flag::Zero => "Z",
            Flag::Signed => "S",
        };

        write!(f, "{as_str}")
    }
}

pub struct State {
    regs: GeneralRegisters,
    pub memory: [u8; MEM_SIZE],
    iptr: usize,
    flags: u16,
    cycles_estimate: u32,
}

impl State {
    pub fn new(stream: &[u8]) -> Self {
        let mut memory = [0; MEM_SIZE];
        memory[..stream.len()].copy_from_slice(&stream[..]);
        // Add a HLT instruction so we know when to stop
        memory[stream.len()] = 0b11110100;

        Self {
            regs: GeneralRegisters::new(),
            memory,
            iptr: 0,
            flags: 0,
            cycles_estimate: 0,
        }
    }

    fn calc_addr(&self, eff_addr: EffAddr) -> usize {
        (eff_addr.base.map_or(0, |r| self.regs.get_reg(r) as i16)
            + eff_addr.index.map_or(0, |r| self.regs.get_reg(r) as i16)
            + eff_addr.offset.unwrap_or(0)) as usize
    }

    pub fn get_value(&self, op: Operand) -> u16 {
        match op {
            Operand::Reg(reg) => self.regs.get_reg(reg),
            Operand::ImmByte(imm) => imm as u16,
            Operand::ImmWord(imm) => imm,
            Operand::MemByte(ea) => self.memory[self.calc_addr(ea)] as u16,
            Operand::MemWord(ea) => {
                let addr = self.calc_addr(ea);
                u16::from_le_bytes([self.memory[addr], self.memory[addr + 1]])
            }
            Operand::RelOffsetByte(_) => todo!(),
        }
    }

    pub fn set_value(&mut self, op: Operand, val: u16) {
        match op {
            Operand::Reg(reg) => self.regs.set_reg(reg, val),
            Operand::ImmByte(_) => panic!("Can't set an immediate value"),
            Operand::ImmWord(_) => panic!("Can't set an immediate value"),
            Operand::MemByte(ea) => self.memory[self.calc_addr(ea)] = val as u8,
            Operand::MemWord(ea) => {
                let addr = self.calc_addr(ea);
                let bytes = val.to_le_bytes();
                self.memory[addr] = bytes[0];
                self.memory[addr + 1] = bytes[1];
            }
            Operand::RelOffsetByte(_) => panic!("Can't set an immediate value"),
        }
    }

    pub fn set_flag(&mut self, flag: Flag) {
        self.flags |= flag as u16;
    }

    pub fn unset_flag(&mut self, flag: Flag) {
        self.flags &= !(flag as u16);
    }

    pub fn is_set(&self, flag: Flag) -> bool {
        (self.flags & flag as u16) > 0
    }

    pub fn flags_as_string(&self) -> String {
        let mut s = String::new();

        if self.is_set(Flag::Parity) {
            s += Flag::Parity.to_string().as_str();
        }

        if self.is_set(Flag::Zero) {
            s += Flag::Zero.to_string().as_str();
        }

        if self.is_set(Flag::Signed) {
            s += Flag::Signed.to_string().as_str();
        }

        s
    }

    pub fn update_flags_from_value(&mut self, val: u16) {
        let before = self.flags_as_string();

        if val == 0 {
            self.set_flag(Flag::Zero);
        } else {
            self.unset_flag(Flag::Zero)
        }

        if val > i16::MAX as u16 {
            self.set_flag(Flag::Signed)
        } else {
            self.unset_flag(Flag::Signed)
        }

        if val.to_le_bytes()[0].count_ones() % 2 == 0 {
            self.set_flag(Flag::Parity)
        } else {
            self.unset_flag(Flag::Parity)
        }

        print!(" flags:{before}->{}", self.flags_as_string())
    }

    pub fn jump(&mut self, op: Operand, condition: bool) {
        if condition {
            let jump_to = match op {
                Operand::Reg(_) => panic!("Cannot jump to a register"),
                Operand::ImmByte(v) => v as usize,
                Operand::ImmWord(v) => v as usize,
                Operand::MemByte(_) => panic!("Cannot jump to memory"),
                Operand::MemWord(_) => panic!("Cannot jump to memory"),
                Operand::RelOffsetByte(r) => self
                    .iptr
                    .checked_add_signed(r as isize)
                    .expect("iptr addtion overflowed"),
            };
            self.iptr = jump_to;
        }
    }

    fn next_instr(&mut self) -> Option<Inst> {
        let Some((n, parsed)) = Inst::from_encoding(&self.memory[self.iptr..]) else {
            return None;
        };

        self.iptr += n;
        return Some(parsed);
    }

    fn dec(&mut self, op: Operand) {
        let dec = self.get_value(op).wrapping_sub(1);

        self.set_value(op, dec);
    }

    fn ea_cycles(ea: EffAddr) -> u32 {
        use Register::*;
        match ea {
           EffAddr { base: None, index: None, offset: Some(_) } => 6,

           EffAddr { base: None, index: Some(_), offset: None }
           | EffAddr { base: Some(_), index: None, offset: None } => 5,

           EffAddr { base: None, index: Some(_), offset: Some(_) }
           | EffAddr { base: Some(_), index: None, offset: Some(_) } => 9,

           EffAddr { base: Some(BP), index: Some(DI), offset: None }
           | EffAddr { base: Some(BX), index: Some(SI), offset: None } => 7,
           EffAddr { base: Some(BP), index: Some(SI), offset: None }
           | EffAddr { base: Some(BX), index: Some(DI), offset: None } => 8,

           EffAddr { base: Some(BP), index: Some(DI), offset: Some(_) }
           | EffAddr { base: Some(BX), index: Some(SI), offset: Some(_) } => 11,
           EffAddr { base: Some(BP), index: Some(SI), offset: Some(_) }
           | EffAddr { base: Some(BX), index: Some(DI), offset: Some(_) } => 12,

           _ => panic!("Invalid EffAddr"),
        }
    }

    fn transfer_penalty(&self, transfers: u32, ea: EffAddr) -> u32 {
        if self.calc_addr(ea) % 2 != 0 {
            transfers * 4
        } else {
            0
        }
    }

    pub fn estimate_cycles(&mut self, inst: &Inst) {
        use Operand::*;
        let (base_cycles, ea_cycles, penality_cycles) = match inst {
            Inst::MOV(op1, op2) => match (op1, op2) {
                (Reg(_), ImmByte(_) | ImmWord(_)) => (4, 0, 0),
                (Reg(_), Reg(_)) => (2, 0, 0),
                (Reg(_), MemByte(ea) | MemWord(ea)) => {
                    (8, Self::ea_cycles(*ea), self.transfer_penalty(1, *ea))
                }
                (MemByte(ea) | MemWord(ea), Reg(_)) => {
                    (9, Self::ea_cycles(*ea), self.transfer_penalty(1, *ea))
                }
                _ => (0, 0, 0)
            },
            Inst::ADD(op1, op2) => match (op1, op2) {
                (Reg(_), ImmByte(_) | ImmWord(_)) => (4, 0, 0),
                (Reg(_), Reg(_)) => (3, 0, 0),
                (Reg(_), MemByte(ea) | MemWord(ea)) => {
                    (9, Self::ea_cycles(*ea), self.transfer_penalty(1, *ea))
                }
                (MemByte(ea) | MemWord(ea), Reg(_)) => {
                    (16, Self::ea_cycles(*ea), self.transfer_penalty(2, *ea))
                }
                (MemByte(ea) | MemWord(ea), ImmByte(_) | ImmWord(_)) => {
                    (17, Self::ea_cycles(*ea), self.transfer_penalty(2, *ea))
                }
                _ => (0, 0, 0)
            },

            Inst::SUB(_, _) => (0, 0, 0),
            Inst::CMP(_, _) => (0, 0, 0),
            Inst::JO(_) => (0, 0, 0),
            Inst::JNO(_) => (0, 0, 0),
            Inst::JB(_) => (0, 0, 0),
            Inst::JNB(_) => (0, 0, 0),
            Inst::JE(_) => (0, 0, 0),
            Inst::JNE(_) => (0, 0, 0),
            Inst::JBE(_) => (0, 0, 0),
            Inst::JNBE(_) => (0, 0, 0),
            Inst::JS(_) => (0, 0, 0),
            Inst::JNS(_) => (0, 0, 0),
            Inst::JP(_) => (0, 0, 0),
            Inst::JNP(_) => (0, 0, 0),
            Inst::JL(_) => (0, 0, 0),
            Inst::JNL(_) => (0, 0, 0),
            Inst::JLE(_) => (0, 0, 0),
            Inst::JNLE(_) => (0, 0, 0),
            Inst::LOOPNZ(_) => (0, 0, 0),
            Inst::LOOPZ(_) => (0, 0, 0),
            Inst::LOOP(_) => (0, 0, 0),
            Inst::JCXZ(_) => (0, 0, 0),
            Inst::HLT => (2, 0, 0),
        };

        let cycles = base_cycles + ea_cycles + penality_cycles;
        self.cycles_estimate += cycles;

        print!(" ; Clocks: +{cycles} = {}", self.cycles_estimate);
        if ea_cycles > 0 || penality_cycles > 0 {
            print!(" ({base_cycles}");

            if ea_cycles > 0 {
                print!(" + {ea_cycles}ea");
            }

            if penality_cycles > 0 {
                print!(" + {penality_cycles}p");
            }

            print!(")");
        }
    }
}

pub fn exec(binary: Vec<u8>) -> State {
    let mut state = State::new(&binary);

    let mut prev_iptr = 0;
    while let Some(inst) = state.next_instr() {
        print!("{inst}");

        state.estimate_cycles(&inst);

        print!(" | ip:0x{prev_iptr:x}->0x{:x}", state.iptr);
        prev_iptr = state.iptr;

        match inst {
            Inst::MOV(op1, op2) => state.set_value(op1, state.get_value(op2)),
            Inst::ADD(op1, op2) => {
                let add = state.get_value(op1) + state.get_value(op2);
                state.set_value(op1, add);
                state.update_flags_from_value(add);
            }
            Inst::SUB(op1, op2) => {
                let sub = state.get_value(op1).wrapping_sub(state.get_value(op2));
                state.set_value(op1, sub);
                state.update_flags_from_value(sub);
            }
            Inst::CMP(op1, op2) => {
                let sub = state.get_value(op1).wrapping_sub(state.get_value(op2));
                state.update_flags_from_value(sub);
            }
            Inst::JO(_op) => todo!(),
            Inst::JNO(_op) => todo!(),
            Inst::JB(_op) => todo!(),
            Inst::JNB(_op) => todo!(),
            Inst::JE(_op) => todo!(),
            Inst::JNE(op) => state.jump(op, !state.is_set(Flag::Zero)),
            Inst::JBE(_op) => todo!(),
            Inst::JNBE(_op) => todo!(),
            Inst::JS(_op) => todo!(),
            Inst::JNS(_op) => todo!(),
            Inst::JP(_op) => todo!(),
            Inst::JNP(_op) => todo!(),
            Inst::JL(_op) => todo!(),
            Inst::JNL(_op) => todo!(),
            Inst::JLE(_op) => todo!(),
            Inst::JNLE(_op) => todo!(),
            Inst::LOOPNZ(_op) => todo!(),
            Inst::LOOPZ(_op) => todo!(),
            Inst::LOOP(op) => {
                state.dec(Operand::Reg(Register::CX));
                state.jump(op, state.get_value(Operand::Reg(Register::CX)) != 0);
            }
            Inst::JCXZ(_op) => todo!(),
            Inst::HLT => {
                println!();
                break;
            }
        }

        println!();
    }

    return state;
}

pub fn exec_file(path: &str) -> State {
    let asm = std::fs::read_to_string(path).expect("Failed to read test file");
    println!("{}", asm);
    let binary = assemble(&asm);
    exec(binary)
}

#[cfg(test)]
mod tests {
    use super::exec_file;
    use crate::parse::Operand::*;
    use crate::parse::Register::*;

    #[test]
    fn test_hw4() {
        println!("Exec imm moves:\n");
        let state = exec_file("inputs/listing_0043_immediate_movs.asm");

        assert_eq!(state.get_value(Reg(AX)), 1);
        assert_eq!(state.get_value(Reg(BX)), 2);
        assert_eq!(state.get_value(Reg(CX)), 3);
        assert_eq!(state.get_value(Reg(DX)), 4);

        assert_eq!(state.get_value(Reg(SP)), 5);
        assert_eq!(state.get_value(Reg(BP)), 6);
        assert_eq!(state.get_value(Reg(SI)), 7);
        assert_eq!(state.get_value(Reg(DI)), 8);

        println!("\nExec reg moves:\n");
        let state = exec_file("inputs/listing_0044_register_movs.asm");

        assert_eq!(state.get_value(Reg(AX)), 4);
        assert_eq!(state.get_value(Reg(BX)), 3);
        assert_eq!(state.get_value(Reg(CX)), 2);
        assert_eq!(state.get_value(Reg(DX)), 1);

        assert_eq!(state.get_value(Reg(SP)), 1);
        assert_eq!(state.get_value(Reg(BP)), 2);
        assert_eq!(state.get_value(Reg(SI)), 3);
        assert_eq!(state.get_value(Reg(DI)), 4);
    }

    #[test]
    fn test_hw5() {
        let state = exec_file("inputs/listing_0046_add_sub_cmp.asm");

        assert_eq!(state.get_value(Reg(BX)), 0xe102);
        assert_eq!(state.get_value(Reg(CX)), 0x0f01);
        assert_eq!(state.get_value(Reg(SP)), 0x03e6);

        assert_eq!(state.flags_as_string(), "PZ");
    }

    #[test]
    fn test_hw6() {
        let state = exec_file("inputs/listing_0048_ip_register.asm");

        assert_eq!(state.get_value(Reg(BX)), 0x07d0);
        assert_eq!(state.get_value(Reg(CX)), 0xfce0);
        assert_eq!(state.iptr, 0x000f);

        assert_eq!(state.flags_as_string(), "S");

        let state = exec_file("inputs/listing_0049_conditional_jumps.asm");

        assert_eq!(state.get_value(Reg(BX)), 0x0406);
        assert_eq!(state.iptr, 0x000f);

        assert_eq!(state.flags_as_string(), "PZ");
    }

    #[test]
    fn test_hw7() {
        let state = exec_file("inputs/listing_0051_memory_mov.asm");

        assert_eq!(state.get_value(Reg(BX)), 1);
        assert_eq!(state.get_value(Reg(CX)), 2);
        assert_eq!(state.get_value(Reg(DX)), 10);
        assert_eq!(state.get_value(Reg(BP)), 4);

        let state = exec_file("inputs/listing_0052_memory_add_loop.asm");

        assert_eq!(state.get_value(Reg(BX)), 6);

        let state = exec_file("inputs/listing_0053_add_loop_challenge.asm");

        assert_eq!(state.get_value(Reg(BX)), 6);
    }

    #[test]
    fn test_hw8() {
        let state = exec_file("inputs/listing_0056_estimating_cycles.asm");

        assert_eq!(state.cycles_estimate, 194);

        let state = exec_file("inputs/listing_0057_challenge_cycles.asm");

        assert_eq!(state.cycles_estimate, 291);
    }
}
