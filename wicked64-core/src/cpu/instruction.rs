#![allow(clippy::unusual_byte_groupings, clippy::upper_case_acronyms)]

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Each CPU instruction consists of a single 32-bit word, aligned on a word
/// boundary. There are three instruction formats: immediate (I-type), jump
/// (J-type), and register (R-type).
///
/// Instructions are encoded as follow (little-endian notation):
///
/// I-Type:
/// ```txt
/// 31    26 25    21 20    16 15            0    (bit)
/// [  op  ] [  rs  ] [  rt  ] [  immediate  ]
/// ```
///
/// J-Type:
/// ```txt
/// 31    26 25         0    (bit)
/// [  op  ] [  target  ]
/// ```
///
/// R-Type:
/// ```txt
/// 31    26 25    21 20    16 15    11 10     6 5         0    (bit)
/// [  op  ] [  rs  ] [  rt  ] [  rd  ] [  sa  ] [  funct  ]
/// ```
///
/// Where:
/// - op: 6-bit operation code
/// - rs: 5-bit source register number
/// - rt: 5-bit target (source/destination) register number or branch
/// condition
/// - immediate: 16-bit immediate value, branch displacement or address
/// displacement
/// - target: 26-bit unconditional branch target address
/// - rd: 5-bit destination register number
/// - sa: 5-bit shift amount
/// - funct: 6-bit function field
#[allow(dead_code, non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    NOP,

    ADDI(ImmediateType),
    ADDIU(ImmediateType),
    ORI(ImmediateType),

    BNE(ImmediateType),
    BGTZ(ImmediateType),
    BGTLZ(ImmediateType),
    BEQ(ImmediateType),
    BLEZ(ImmediateType),
    BLEZL(ImmediateType),
    BEQL(ImmediateType),
    BNEL(ImmediateType),
    BGTZL(ImmediateType),

    J(JumpType),
    JAL(JumpType),

    SW(ImmediateType),

    LUI(ImmediateType),
    LW(ImmediateType),
    LB(ImmediateType),
    LH(ImmediateType),
    LWL(ImmediateType),
    LBU(ImmediateType),
    LHU(ImmediateType),
    LWR(ImmediateType),
    LWU(ImmediateType),

    // Special instructions
    SpecialSLL(RegisterType),
    SpecialSRL(RegisterType),
    SpecialSRA(RegisterType),
    SpecialSLLV(RegisterType),
    SpecialSRLV(RegisterType),
    SpecialSRAV(RegisterType),
    SpecialJR(RegisterType),
    SpecialJALR(RegisterType),
    SpecialSYSCALL(RegisterType),
    SpecialBREAK(RegisterType),
    SpecialSYNC(RegisterType),
    SpecialMFHI(RegisterType),
    SpecialMTHI(RegisterType),
    SpecialMFLO(RegisterType),
    SpecialMTLO(RegisterType),
    SpecialDSLLV(RegisterType),
    SpecialDSRLV(RegisterType),
    SpecialDSRAV(RegisterType),
    SpecialMULT(RegisterType),
    SpecialMULTU(RegisterType),
    SpecialDIV(RegisterType),
    SpecialDIVU(RegisterType),
    SpecialDMULT(RegisterType),
    SpecialDMULTU(RegisterType),
    SpecialDDIV(RegisterType),
    SpecialDDIVU(RegisterType),
    SpecialADD(RegisterType),
    SpecialADDU(RegisterType),
    SpecialSUB(RegisterType),
    SpecialSUBU(RegisterType),
    SpecialAND(RegisterType),
    SpecialOR(RegisterType),
    SpecialXOR(RegisterType),
    SpecialNOR(RegisterType),
    SpecialSLT(RegisterType),
    SpecialSLTU(RegisterType),
    SpecialDADD(RegisterType),
    SpecialDADDU(RegisterType),
    SpecialDSUB(RegisterType),
    SpecialDSUBU(RegisterType),
    SpecialTGE(RegisterType),
    SpecialTGEU(RegisterType),
    SpecialTLT(RegisterType),
    SpecialTLTU(RegisterType),
    SpecialTEQ(RegisterType),
    SpecialTNE(RegisterType),
    SpecialDSLL(RegisterType),
    SpecialDSRL(RegisterType),
    SpecialDSRA(RegisterType),
    SpecialDSLL32(RegisterType),
    SpecialDSRL32(RegisterType),
    SpecialDSRA32(RegisterType),

    // COP0 instructions
    Cop0DMFC0(RegisterType),
    Cop0DMTC0(RegisterType),
    Cop0MFC0(RegisterType),
    Cop0MTC0(RegisterType),
    Cop0ERET(RegisterType),
    Cop0TLBP(RegisterType),
    Cop0TLBR(RegisterType),
    Cop0TLBWI(RegisterType),
    Cop0TLBWR(RegisterType),
}

impl Instruction {
    /// Decode a SPECIAL instruction
    fn decode_special(instruction: u32) -> anyhow::Result<Instruction> {
        let rtype = RegisterType::new(instruction);

        match SpecialFunct::try_from(rtype.funct) {
            Ok(funct) => match funct {
                SpecialFunct::SLL => Ok(Instruction::SpecialSLL(rtype)),
                SpecialFunct::SRL => Ok(Instruction::SpecialSRL(rtype)),
                SpecialFunct::SRA => Ok(Instruction::SpecialSRA(rtype)),
                SpecialFunct::SLLV => Ok(Instruction::SpecialSLLV(rtype)),
                SpecialFunct::SRLV => Ok(Instruction::SpecialSRLV(rtype)),
                SpecialFunct::SRAV => Ok(Instruction::SpecialSRAV(rtype)),
                SpecialFunct::JR => Ok(Instruction::SpecialJR(rtype)),
                SpecialFunct::JALR => Ok(Instruction::SpecialJALR(rtype)),
                SpecialFunct::SYSCALL => Ok(Instruction::SpecialSYSCALL(rtype)),
                SpecialFunct::BREAK => Ok(Instruction::SpecialBREAK(rtype)),
                SpecialFunct::SYNC => Ok(Instruction::SpecialSYNC(rtype)),
                SpecialFunct::MFHI => Ok(Instruction::SpecialMFHI(rtype)),
                SpecialFunct::MTHI => Ok(Instruction::SpecialMTHI(rtype)),
                SpecialFunct::MFLO => Ok(Instruction::SpecialMFLO(rtype)),
                SpecialFunct::MTLO => Ok(Instruction::SpecialMTLO(rtype)),
                SpecialFunct::DSLLV => Ok(Instruction::SpecialDSLLV(rtype)),
                SpecialFunct::DSRLV => Ok(Instruction::SpecialDSRLV(rtype)),
                SpecialFunct::DSRAV => Ok(Instruction::SpecialDSRAV(rtype)),
                SpecialFunct::MULT => Ok(Instruction::SpecialMULT(rtype)),
                SpecialFunct::MULTU => Ok(Instruction::SpecialMULTU(rtype)),
                SpecialFunct::DIV => Ok(Instruction::SpecialDIV(rtype)),
                SpecialFunct::DIVU => Ok(Instruction::SpecialDIVU(rtype)),
                SpecialFunct::DMULT => Ok(Instruction::SpecialDMULT(rtype)),
                SpecialFunct::DMULTU => Ok(Instruction::SpecialDMULTU(rtype)),
                SpecialFunct::DDIV => Ok(Instruction::SpecialDDIV(rtype)),
                SpecialFunct::DDIVU => Ok(Instruction::SpecialDDIVU(rtype)),
                SpecialFunct::ADD => Ok(Instruction::SpecialADD(rtype)),
                SpecialFunct::ADDU => Ok(Instruction::SpecialADDU(rtype)),
                SpecialFunct::SUB => Ok(Instruction::SpecialSUB(rtype)),
                SpecialFunct::SUBU => Ok(Instruction::SpecialSUBU(rtype)),
                SpecialFunct::AND => Ok(Instruction::SpecialAND(rtype)),
                SpecialFunct::OR => Ok(Instruction::SpecialOR(rtype)),
                SpecialFunct::XOR => Ok(Instruction::SpecialXOR(rtype)),
                SpecialFunct::NOR => Ok(Instruction::SpecialNOR(rtype)),
                SpecialFunct::SLT => Ok(Instruction::SpecialSLT(rtype)),
                SpecialFunct::SLTU => Ok(Instruction::SpecialSLTU(rtype)),
                SpecialFunct::DADD => Ok(Instruction::SpecialDADD(rtype)),
                SpecialFunct::DADDU => Ok(Instruction::SpecialDADDU(rtype)),
                SpecialFunct::DSUB => Ok(Instruction::SpecialDSUB(rtype)),
                SpecialFunct::DSUBU => Ok(Instruction::SpecialDSUBU(rtype)),
                SpecialFunct::TGE => Ok(Instruction::SpecialTGE(rtype)),
                SpecialFunct::TGEU => Ok(Instruction::SpecialTGEU(rtype)),
                SpecialFunct::TLT => Ok(Instruction::SpecialTLT(rtype)),
                SpecialFunct::TLTU => Ok(Instruction::SpecialTLTU(rtype)),
                SpecialFunct::TEQ => Ok(Instruction::SpecialTEQ(rtype)),
                SpecialFunct::TNE => Ok(Instruction::SpecialTNE(rtype)),
                SpecialFunct::DSLL => Ok(Instruction::SpecialDSLL(rtype)),
                SpecialFunct::DSRL => Ok(Instruction::SpecialDSRL(rtype)),
                SpecialFunct::DSRA => Ok(Instruction::SpecialDSRA(rtype)),
                SpecialFunct::DSLL32 => Ok(Instruction::SpecialDSLL32(rtype)),
                SpecialFunct::DSRL32 => Ok(Instruction::SpecialDSRL32(rtype)),
                SpecialFunct::DSRA32 => Ok(Instruction::SpecialDSRA32(rtype)),
            },
            Err(_) => anyhow::bail!("Unknown Special instruction: 0x{instruction:08x}"),
        }
    }

    /// Decode a COP0 instruction.
    ///
    /// There are 9 COP0 instructions, which can divided in two distinct groups:
    /// ```txt
    /// First group (CO=0):
    ///           COP0    instruction
    /// DMFC0 |> 010_000 | 00001[DMF] | rt | rd | 0*11
    /// DMTC0 |> 010_000 | 00101[DMT] | rt | rd | 0*11
    /// MFC0  |> 010_000 | 00000[MF ] | rt | rd | 0*11
    /// MTC0  |> 010_000 | 00100[MT ] | rt | rd | 0*11
    /// Second group (CO=1):
    ///           COP0     group          instruction
    /// ERET  |> 010_000 | 1[CO] | 0*19 | 011_000[ERET ]
    /// TLBP  |> 010_000 | 1[CO] | 0*19 | 001_000[TLBP ]
    /// TLBR  |> 010_000 | 1[CO] | 0*19 | 000_001[TLBR ]
    /// TLBWI |> 010_000 | 1[CO] | 0*19 | 000_010[TLBWI]
    /// TLBWR |> 010_000 | 1[CO] | 0*19 | 000_110[TLBWR]
    /// ```
    fn decode_cop0(instruction: u32) -> anyhow::Result<Instruction> {
        let rtype = RegisterType::new(instruction);
        // check if "CO" (i.e: bit 4 of `rs`) is 1
        let decoded = match rtype.rs & 0x10 == 0x10 {
            true => match Cop0Funct::try_from(rtype.funct) {
                Ok(Cop0Funct::ERET) => Some(Self::Cop0ERET(rtype)),
                Ok(Cop0Funct::TLBP) => Some(Self::Cop0TLBP(rtype)),
                Ok(Cop0Funct::TLBR) => Some(Self::Cop0TLBR(rtype)),
                Ok(Cop0Funct::TLBWI) => Some(Self::Cop0TLBWI(rtype)),
                Ok(Cop0Funct::TLBWR) => Some(Self::Cop0TLBWR(rtype)),
                Err(_) => None,
            },
            false => match Cop0RS::try_from(rtype.rs) {
                Ok(Cop0RS::DMFC0) => Some(Self::Cop0DMFC0(rtype)),
                Ok(Cop0RS::DMTC0) => Some(Self::Cop0DMTC0(rtype)),
                Ok(Cop0RS::MFC0) => Some(Self::Cop0MFC0(rtype)),
                Ok(Cop0RS::MTC0) => Some(Self::Cop0MTC0(rtype)),
                Err(_) => None,
            },
        };

        decoded.ok_or_else(|| anyhow::anyhow!("Unknown COP0 instruction: 0x{instruction:08x}"))
    }

    pub fn cycles(&self) -> usize {
        #[allow(clippy::match_single_binding)]
        match self {
            _ => 5,
        }
    }
}

impl TryFrom<u32> for Instruction {
    type Error = anyhow::Error;

    fn try_from(instruction: u32) -> Result<Self, Self::Error> {
        if instruction == 0 {
            return Ok(Self::NOP);
        }

        let opcode = Opcode::try_from((instruction >> 26) as u8);
        match opcode {
            Ok(opcode) => match opcode {
                Opcode::ADDI => Ok(Self::ADDI(ImmediateType::new(instruction))),
                Opcode::ADDIU => Ok(Self::ADDIU(ImmediateType::new(instruction))),
                Opcode::ORI => Ok(Self::ORI(ImmediateType::new(instruction))),

                Opcode::BNE => Ok(Self::BNE(ImmediateType::new(instruction))),
                Opcode::BEQ => Ok(Self::BEQ(ImmediateType::new(instruction))),
                Opcode::BLEZ => Ok(Self::BLEZ(ImmediateType::new(instruction))),
                Opcode::BGTZ => Ok(Self::BGTZ(ImmediateType::new(instruction))),
                Opcode::BEQL => Ok(Self::BEQL(ImmediateType::new(instruction))),
                Opcode::BNEL => Ok(Self::BNEL(ImmediateType::new(instruction))),
                Opcode::BLEZL => Ok(Self::BLEZL(ImmediateType::new(instruction))),
                Opcode::BGTZL => Ok(Self::BGTZL(ImmediateType::new(instruction))),

                Opcode::J => Ok(Self::J(JumpType::new(instruction))),
                Opcode::JAL => Ok(Self::JAL(JumpType::new(instruction))),

                Opcode::SW => Ok(Self::SW(ImmediateType::new(instruction))),

                Opcode::LUI => Ok(Self::LUI(ImmediateType::new(instruction))),
                Opcode::LW => Ok(Self::LW(ImmediateType::new(instruction))),
                Opcode::LB => Ok(Self::LB(ImmediateType::new(instruction))),
                Opcode::LH => Ok(Self::LH(ImmediateType::new(instruction))),
                Opcode::LWL => Ok(Self::LWL(ImmediateType::new(instruction))),
                Opcode::LBU => Ok(Self::LBU(ImmediateType::new(instruction))),
                Opcode::LHU => Ok(Self::LHU(ImmediateType::new(instruction))),
                Opcode::LWR => Ok(Self::LWR(ImmediateType::new(instruction))),
                Opcode::LWU => Ok(Self::LWU(ImmediateType::new(instruction))),

                Opcode::SPECIAL => Self::decode_special(instruction),
                Opcode::COP0 => Self::decode_cop0(instruction),
                _ => anyhow::bail!(
                    "Unhandled opcode '{opcode:?}' from instruction 0x{instruction:08x}"
                ),
            },
            Err(_) => anyhow::bail!("Unknown instruction 0x{instruction:08x}"),
        }
    }
}

/// I-type instruction
#[derive(Debug, Clone, Copy)]
pub struct ImmediateType {
    pub opcode: u8, // 6 bits
    pub rs: u8,     // 5 bits
    pub rt: u8,     // 5 bits
    pub imm: u16,   // 16 bits
}

impl ImmediateType {
    #[allow(dead_code)]
    fn new(instruction: u32) -> ImmediateType {
        Self {
            opcode: (instruction >> 26) as u8,
            rs: ((instruction >> 21) & 0x1f) as u8,
            rt: ((instruction >> 16) & 0x1f) as u8,
            imm: instruction as u16,
        }
    }
}

/// J-type instruction
#[derive(Debug, Clone, Copy)]
pub struct JumpType {
    pub opcode: u8,  // 6 bits
    pub target: u32, // 26 bits
}

impl JumpType {
    #[allow(dead_code)]
    fn new(instruction: u32) -> JumpType {
        Self {
            opcode: (instruction >> 26) as u8,
            target: (instruction & 0x1ff_ffff) as u32,
        }
    }
}

/// R-type instruction
#[derive(Debug, Clone, Copy)]
pub struct RegisterType {
    pub opcode: u8,       // 6 bits
    pub rs: u8,           // 5 bits
    pub rt: u8,           // 5 bits
    pub rd: u8,           // 5 bits
    pub shift_amount: u8, // 5 bits
    pub funct: u8,        // 6 bits
}

impl RegisterType {
    fn new(instruction: u32) -> RegisterType {
        Self {
            opcode: (instruction >> 26) as u8,
            rs: ((instruction >> 21) & 0x1f) as u8,
            rt: ((instruction >> 16) & 0x1f) as u8,
            rd: ((instruction >> 11) & 0x1f) as u8,
            shift_amount: ((instruction >> 6) & 0x1f) as u8,
            funct: (instruction & 0x1f) as u8,
        }
    }
}

/// N64 opcodes
/// Refer to <https://www.zophar.net/fileuploads/2/10655uytsm/N64ops03.txt>
#[repr(u8)]
#[rustfmt::skip]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
enum Opcode {
    SPECIAL = 0b000_000,
    REGIMM  = 0b000_001,
    J       = 0b000_010,
    JAL     = 0b000_011,
    BEQ     = 0b000_100,
    BNE     = 0b000_101,
    BLEZ    = 0b000_110,
    BGTZ    = 0b000_111,

    ADDI    = 0b001_000,
    ADDIU   = 0b001_001,
    SLTI    = 0b001_010,
    SLTIU   = 0b001_011,
    ANDI    = 0b001_100,
    ORI     = 0b001_101,
    XORI    = 0b001_110,
    LUI     = 0b001_111,

    COP0    = 0b010_000,
    COP1    = 0b010_001,
    //      = 0b010_010,
    //      = 0b010_011,
    BEQL    = 0b010_100,
    BNEL    = 0b010_101,
    BLEZL   = 0b010_110,
    BGTZL   = 0b010_111,

    DADDI   = 0b011_000,
    DADDIU  = 0b011_001,
    LDL     = 0b011_010,
    LDR     = 0b011_011,
    //      = 0b001_100,
    //      = 0b001_101,
    //      = 0b001_110,
    //      = 0b001_111,

    LB      = 0b100_000,
    LH      = 0b100_001,
    LWL     = 0b100_010,
    LW      = 0b100_011,
    LBU     = 0b100_100,
    LHU     = 0b100_101,
    LWR     = 0b100_110,
    LWU     = 0b100_111,

    SB      = 0b101_000,
    SH      = 0b101_001,
    SWL     = 0b101_010,
    SW      = 0b101_011,
    SDL     = 0b101_100,
    SDR     = 0b101_101,
    SWR     = 0b101_110,
    CACHE   = 0b101_111,

    LL      = 0b110_000,
    LWC1    = 0b110_001,
    LWC2    = 0b110_010,
    //      = 0b110_011,
    LLD     = 0b110_100,
    LDC1    = 0b110_101,
    LDC2    = 0b110_110,
    LD      = 0b110_111,

    SC      = 0b111_000,
    SWC1    = 0b111_001,
    SWC2    = 0b111_010,
    //      = 0b111_011,
    SCD     = 0b111_100,
    SDC1    = 0b111_101,
    SDC2    = 0b111_110,
    SD      = 0b111_111,
}

/// N64 Special instruction `funct` bits
#[repr(u8)]
#[rustfmt::skip]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
enum SpecialFunct {
    SLL     = 0b000_000,
    //      = 0b000_001,
    SRL     = 0b000_010,
    SRA     = 0b000_011,
    SLLV    = 0b000_100,
    //      = 0b000_101,
    SRLV    = 0b000_110,
    SRAV    = 0b000_111,

    JR      = 0b001_000,
    JALR    = 0b001_001,
    //      = 0b001_010,
    //      = 0b001_011,
    SYSCALL = 0b001_100,
    BREAK   = 0b001_101,
    //      = 0b001_110,
    SYNC    = 0b001_111,

    MFHI    = 0b010_000,
    MTHI    = 0b010_001,
    MFLO    = 0b010_010,
    MTLO    = 0b010_011,
    DSLLV   = 0b010_100,
    //      = 0b010_101,
    DSRLV   = 0b010_110,
    DSRAV   = 0b010_111,

    MULT    = 0b011_000,
    MULTU   = 0b011_001,
    DIV     = 0b011_010,
    DIVU    = 0b011_011,
    DMULT   = 0b011_100,
    DMULTU  = 0b011_101,
    DDIV    = 0b011_110,
    DDIVU   = 0b011_111,

    ADD     = 0b100_000,
    ADDU    = 0b100_001,
    SUB     = 0b100_010,
    SUBU    = 0b100_011,
    AND     = 0b100_100,
    OR      = 0b100_101,
    XOR     = 0b100_110,
    NOR     = 0b100_111,

    //      = 0b101_000,
    //      = 0b101_001,
    SLT     = 0b101_010,
    SLTU    = 0b101_011,
    DADD    = 0b101_100,
    DADDU   = 0b101_101,
    DSUB    = 0b101_110,
    DSUBU   = 0b101_111,

    TGE     = 0b110_000,
    TGEU    = 0b110_001,
    TLT     = 0b110_010,
    TLTU    = 0b110_011,
    TEQ     = 0b110_100,
    //      = 0b110_101,
    TNE     = 0b110_110,
    //      = 0b110_111,

    DSLL    = 0b111_000,
    //      = 0b111_001,
    DSRL    = 0b111_010,
    DSRA    = 0b111_011,
    DSLL32  = 0b111_100,
    //      = 0b111_101,
    DSRL32  = 0b111_110,
    DSRA32  = 0b111_111,
}

/// N64 COP0 instruction `funct` bits
#[repr(u8)]
#[rustfmt::skip]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
enum Cop0Funct {
    ERET  = 0b011_000,
    TLBP  = 0b001_000,
    TLBR  = 0b000_001,
    TLBWI = 0b000_010,
    TLBWR = 0b000_110,
}

/// N64 COP0 instruction `rs` bits
#[repr(u8)]
#[rustfmt::skip]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
pub enum Cop0RS {
    DMFC0 = 0b00001,
    DMTC0 = 0b00101,
    MFC0  = 0b00000,
    MTC0  = 0b00100,
}
