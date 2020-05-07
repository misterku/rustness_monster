use hex;
use std::num::Wrapping;
use byteorder::{ByteOrder, LittleEndian};
use crate::opscode;
use std::collections::HashMap;

bitflags! {

/// # Status Register (P)
///
///  7 6 5 4 3 2 1 0
///  N V _ B D I Z C
///  | |   | | | | +--- Carry Flag
///  | |   | | | +----- Zero Flag
///  | |   | | +------- Interrupt Disable 
///  | |   | +--------- Decimal Mode (Allows BCD, not implemented on NES)
///  | |   +----------- Break Command
///  | +--------------- Overflow Flag
///  +----------------- Negative Flag
///     
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const OVERFLOW          = 0b01000000;
        const NEGATIV           = 0b10000000;
    }
}

struct Memory {

    space: [u8; 0xffff],
}

/// # Memory Map http://nesdev.com/NESDoc.pdf
/// 
///  _______________ $10000  _______________
/// | PRG-ROM       |       |               |
/// | Upper Bank    |       |               |
/// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
/// | PRG-ROM       |       |               |
/// | Lower Bank    |       |               |
/// |_______________| $8000 |_______________|
/// | SRAM          |       | SRAM          |
/// |_______________| $6000 |_______________|
/// | Expansion ROM |       | Expansion ROM |
/// |_______________| $4020 |_______________|
/// | I/O Registers |       |               |
/// |_ _ _ _ _ _ _ _| $4000 |               |
/// | Mirrors       |       | I/O Registers |
/// | $2000-$2007   |       |               |
/// |_ _ _ _ _ _ _ _| $2008 |               |
/// | I/O Registers |       |               |
/// |_______________| $2000 |_______________|
/// | Mirrors       |       |               |
/// | $0000-$07FF   |       |               |
/// |_ _ _ _ _ _ _ _| $0800 |               |
/// | RAM           |       | RAM           |
/// |_ _ _ _ _ _ _ _| $0200 |               |
/// | Stack         |       |               |
/// |_ _ _ _ _ _ _ _| $0100 |               |
/// | Zero Page     |       |               |
/// |_______________| $0000 |_______________|
/// 
trait Mem {
    const ZERO_PAGE: u16 = 0x0;
    const STACK: u16 = 0x0100;
    const RAM: u16 = 0x0200;
    const RAM_MIRRORS: u16 = 0x0800;
    const IO_REGISTERS: u16 = 0x2000;
    const IO_MIRRORS: u16 = 0x2008;

    fn write(&mut self, pos: u16, data: u8);
    fn read(&self, pos: u16) -> u8;
    fn read_u16(&self, pos: u16) -> u16;
}

impl Mem for Memory {
    fn write(&mut self, pos: u16, data: u8) {
        self.space[pos as usize] = data
    }

    fn read(&self, pos: u16) -> u8 {
        self.space[pos as usize]
    }

    fn read_u16(&self, pos: u16) -> u16 {
        LittleEndian::read_u16(&self.space[pos as usize..])    
    }
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            space: [0; 0xFFFF]
        }
    }

}

pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,   
    None_Addressing,
}

impl AddressingMode {
    pub fn read_u8(&self, mem: &[u8], cpu: &CPU) -> u8 {
        let pos: u8 = mem[cpu.program_counter as usize];
        match self {
            Immediate => pos,
            ZeroPage => cpu.memory.read(pos as u16),
            ZeroPage_X=> cpu.memory.read((pos + cpu.register_x) as u16),
            ZeroPage_Y=> cpu.memory.read((pos + cpu.register_y) as u16),
            Absolute => {
                let mem_address = LittleEndian::read_u16(&mem[pos as usize..]);
                cpu.memory.read(mem_address)
            },
            Absolute_X => {
                let mem_address = LittleEndian::read_u16(&mem[pos as usize..]) + cpu.register_x as u16;
                cpu.memory.read(mem_address)
            },
            Absolute_Y => {
                let mem_address = LittleEndian::read_u16(&mem[pos as usize..]) + cpu.register_y as u16;
                cpu.memory.read(mem_address)
            },
            Indirect_X => {
                let ptr: u8 = pos + cpu.register_x ; //todo overflow
                let deref = cpu.memory.read_u16(ptr as u16);
                cpu.memory.read(deref)
            },
            Indirect_Y => {
                let deref = cpu.memory.read_u16(pos as u16) + cpu.register_y as u16;
                cpu.memory.read(deref)
            },
            None_Addressing => panic!("AddressingMode::NoneAddressing shouldn't be used to read data"),
        }
    }  

    pub fn write_u8(&self, mem: &[u8], cpu: &mut CPU, data: u8) {
        let pos: u8 = mem[cpu.program_counter as usize];
      
        match self {
            Immediate => panic!("Immidiate adressing mode only for reading"),
            ZeroPage => cpu.memory.write(pos as u16, data),
            ZeroPage_X=> cpu.memory.write((pos + cpu.register_x) as u16, data),
            ZeroPage_Y=> cpu.memory.write((pos + cpu.register_y) as u16, data),
            Absolute => {
                let mem_address = LittleEndian::read_u16(&mem[pos as usize..]);
                cpu.memory.write(mem_address, data)
            },
            Absolute_X => {
                let mem_address = LittleEndian::read_u16(&mem[pos as usize..]) + cpu.register_x as u16;
                cpu.memory.write(mem_address, data)
            },
            Absolute_Y => {
                let mem_address = LittleEndian::read_u16(&mem[pos as usize..]) + cpu.register_y as u16;
                cpu.memory.write(mem_address, data)
            },
            Indirect_X => {
                let ptr: u8 = pos + cpu.register_x ; //todo overflow
                let deref = cpu.memory.read_u16(ptr as u16);
                cpu.memory.write(deref, data)
            },
            Indirect_Y => {
                let deref = cpu.memory.read_u16(pos as u16) + cpu.register_y as u16;
                cpu.memory.write(deref, data)
            },
            None_Addressing => panic!("AddressingMode::NoneAddressing shouldn't be used to read data"),
        }
    }
}

pub struct CPU {
    register_a: u8,
    register_x: u8,
    register_y: u8,
    stack_pointer: u8,
    program_counter: u16, 
    flags: CpuFlags,
    memory: Memory,
}

impl CPU {
    pub fn transform(s: &str) -> Vec<u8> {
        hex::decode(s.replace(' ', "")).expect("Decoding failed")
    }

    fn set_register_a(&mut self, data: u8) {
        self.register_a = data;
        if self.register_a == 0  {
            self.flags.insert(CpuFlags::ZERO);  
        } else {
            self.flags.remove(CpuFlags::ZERO);
        }
        if self.register_a | 0b10000000 == 1 {
            self.flags.insert(CpuFlags::NEGATIV)
        } else {
            self.flags.remove(CpuFlags::NEGATIV)
        }
    }

    fn set_carry_flag(&mut self) {
        self.flags.insert(CpuFlags::CARRY)
    }

    fn clear_carry_flag(&mut self) {
        self.flags.remove(CpuFlags::CARRY)    
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.memory.read((Memory::STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.memory.write((Memory::STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    pub fn interpret(&mut self, program: Vec<u8>) {
        let ref opscodes: HashMap<u8, &'static opscode::OpsCode>  = *opscode::OPSCODES_MAP;

        let begin = self.program_counter as usize;
        self.program_counter += 1;
        match program[begin] {
            0x18 /*CLC*/ => {
                self.clear_carry_flag();
            }
            0x38 /*SEC*/ => {
                self.set_carry_flag();
            }, 

            0x48 /* PHA */ => {
                self.stack_push(self.register_a);
            },
            0x68 /* PLA */ => {
                let data = self.stack_pop();
                self.set_register_a(data);
            },
            0x85 /*STA Zero Page*/ => {
                let pos: u8 = program[begin +1];   
                self.memory.write(pos as u16, self.register_a);
                // self.program_counter += 1; 

            },
            0x95 /*STA Zero Page,X*/ => {
                let pos: u8 = program[begin +1] + self.register_x;    //todo overflow? 
                self.memory.write(pos as u16, self.register_a);
                // self.program_counter += 1; 
            },
            0x8d /*STA Absolute*/ => {
                let pos = LittleEndian::read_u16(&program[(begin+1) as usize..]);
                self.memory.write(pos, self.register_a);
                // self.program_counter += 2
            }, 
            0x9d /*STA Absolute,X*/ => {
                let pos = LittleEndian::read_u16(&program[(begin+1) as usize..]) + self.register_x as u16;
                self.memory.write(pos, self.register_a);
                // self.program_counter += 2
            },
            0x99 /*STA Absolute,Y*/ => {
                let pos = LittleEndian::read_u16(&program[(begin+1) as usize..]) + self.register_y as u16;
                self.memory.write(pos, self.register_a);
                // self.program_counter += 2
            },

            0x81 /*STA (Indirect,X)*/ => {
                let ptr: u8 = program[begin +1] + self.register_x ; //todo overflow

                let deref = self.memory.read_u16(ptr as u16);
                self.memory.write(deref, self.register_a);
                // self.program_counter += 1
            },
            
            0x91 /*STA (Indirect), Y*/ => {
                let ptr: u8 = program[begin +1] ; //todo overflow

                let deref = self.memory.read_u16(ptr as u16) + self.register_y as u16;
                self.memory.write(deref, self.register_a);
                // self.program_counter += 1
            },

            // 0xa9 /* LDA Immidiate */ => {
            //     let data  = AddressingMode::Immediate.read_u8(&program[..], self);
            //     self.set_register_a(data);
            // },
            0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 /* LDA */ => {
                // let data  = AddressingMode::Immediate.read_u8(&program[..], self);
                let ops = opscodes.get(&program[begin]).unwrap();
                let data = ops.mode.read_u8(&program[..], self);
                self.set_register_a(data);
                // self.program_counter += 1;
                // self.set_register_a(program[(begin + 1) as usize]);
                // self.program_counter += 2
            },
            _ => { panic!("Unknown ops code") }
        }
        // &HashMap<u8, &'static opscode::OpsCode>*/
        if let Some(&ops) = opscodes.get(&program[begin]) {
            self.program_counter += (ops.len - 1) as u16;
            //todo: cycles
        } else {
            //todo: panic
        }
        
        if (self.program_counter as usize) < program.len() {
            self.interpret(program)
        }

    }

    pub fn new() -> Self {
        return CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: 0xFF, 
            program_counter: 0,
            flags: CpuFlags::from_bits_truncate(0b00100000),
            memory: Memory::new()
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transform() {
        assert_eq!(CPU::transform("a9 8d"), [169, 141]);
    }

    #[test]
    fn test_0xa9_load_into_register_a() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 8d"));
        assert_eq!(cpu.register_a, 0x8d);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_larger_program() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 01 8d 00 02 a9 05 8d 01 02 a9 08 8d 02 02"));
        assert_eq!(cpu.memory.read(0x0200), 01);
        assert_eq!(cpu.memory.read(0x0201), 05);
        assert_eq!(cpu.memory.read(0x0202), 08);
        assert_eq!(cpu.program_counter, 15);
    }

    #[test]
    fn test_0x48_pha() {
        let mut cpu = CPU::new();
        cpu.register_a = 100;
        cpu.interpret(CPU::transform("48"));
        assert_eq!(cpu.stack_pointer, 0xFE);
        assert_eq!(cpu.memory.read(Memory::STACK + 0xFF), 100);
        assert_eq!(cpu.program_counter, 1);
    }

    #[test]
    fn test_0x68_pla(){
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 ff 48 a9 00 68"));
        assert_eq!(cpu.stack_pointer, 0xFF);
        assert_eq!(cpu.register_a, 0xff);
        assert_eq!(cpu.program_counter, 6);
    }

    #[test]
    fn test_0x48_pla_flags() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 00 48 a9 01 68"));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_stack_overflowing() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("68"));
    }

    #[test]
    fn test_0x18_clc() {
        let mut cpu = CPU::new();
        cpu.flags.insert(CpuFlags::CARRY);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        cpu.interpret(CPU::transform("18"));
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert_eq!(cpu.program_counter, 1);
    }

    #[test]
    fn test_0x38_sec() {
        let mut cpu = CPU::new();
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        cpu.interpret(CPU::transform("38"));
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert_eq!(cpu.program_counter, 1); 
    }

    #[test]
    fn test_0x85_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.interpret(CPU::transform("85 10"));
        assert_eq!(cpu.memory.read(0x10), 101);
        assert_eq!(cpu.program_counter, 2);
    } 
    
    #[test]
    fn test_0x95_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.register_x = 0x50;
        cpu.interpret(CPU::transform("95 10"));
        assert_eq!(cpu.memory.read(0x60), 101);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_0x8d_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 100;
        cpu.interpret(CPU::transform("8d 00 02"));
        assert_eq!(cpu.memory.read(0x0200), 100);
        assert_eq!(cpu.program_counter, 3);
    }

    #[test]
    fn test_0x9d_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.register_x = 0x50;
        cpu.interpret(CPU::transform("9d 00 11"));
        assert_eq!(cpu.memory.read(0x1150), 101);
        assert_eq!(cpu.program_counter, 3);
    } 
    
    #[test]
    fn test_0x99_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.register_y = 0x66;
        cpu.interpret(CPU::transform("99 00 11"));
        assert_eq!(cpu.memory.read(0x1166), 101);
        assert_eq!(cpu.program_counter, 3);
    }

    #[test]
    fn test_0x81_sta() {
        let mut cpu = CPU::new();    
        cpu.register_x = 2;
        cpu.memory.write(0x2, 0x05);
        cpu.memory.write(0x3, 0x07);

        cpu.register_a=0x66;

        cpu.interpret(CPU::transform("81 00"));
        assert_eq!(cpu.memory.read(0x0705), 0x66);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_091_sta() {
        let mut cpu = CPU::new();    
        cpu.register_y = 0x10;
        cpu.memory.write(0x2, 0x05);
        cpu.memory.write(0x3, 0x07);

        cpu.register_a=0x66;

        cpu.interpret(CPU::transform("91 02"));
        assert_eq!(cpu.memory.read(0x0705 + 0x10), 0x66);
        assert_eq!(cpu.program_counter, 2);
    }
}