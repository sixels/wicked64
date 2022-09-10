use pretty_assertions::assert_eq;

use w64_codegen::{emit, Emitter};

#[path = "./common.rs"]
mod common;

use common::{dump_file, registers};

#[test]
fn add_reg_reg() {
    let mut emitter = Emitter::default();

    let regs = registers();
    for dst in regs.iter().copied() {
        for src in regs.iter().copied() {
            emit!(emitter, add %dst, %src);
        }
    }

    assert_eq!(emitter.as_slice(), dump_file("alu/add_reg_reg"));
}
#[test]
fn add_reg_imm() {
    let mut emitter = Emitter::default();

    let regs = registers();
    let val = 0x78563412;
    for dst in regs.iter().copied() {
        emit!(emitter, add %dst, $val);
    }

    assert_eq!(emitter.as_slice(), dump_file("alu/add_reg_imm"));
}

#[test]
fn sub_reg_reg() {
    let mut emitter = Emitter::default();
    let regs = registers();
    for dst in regs.iter().copied() {
        for src in regs.iter().copied() {
            emit!(emitter, sub %dst, %src);
        }
    }

    assert_eq!(emitter.as_slice(), dump_file("alu/sub_reg_reg"));
}
#[test]
fn sub_reg_imm() {
    let mut emitter = Emitter::default();

    let regs = registers();
    let val = 0x78563412;
    for dst in regs.iter().copied() {
        emit!(emitter, sub %dst, $val);
    }

    assert_eq!(emitter.as_slice(), dump_file("alu/sub_reg_imm"));
}
