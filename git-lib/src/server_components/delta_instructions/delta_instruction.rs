use std::io::{Read, Write};

use crate::command_errors::CommandError;

use super::{copy_instruction::CopyInstruction, data_instruction::DataInstruction};

pub type DeltaInstruction = Box<dyn DeltaInstructionTrait>;

pub trait DeltaInstructionTrait: std::fmt::Debug {
    fn apply(&self, new_object: &mut dyn Write, base_object: &Vec<u8>) -> Result<(), CommandError>;
    fn read_from(first_byte: u8, stream: &mut dyn Read) -> Result<DeltaInstruction, CommandError>
    where
        Self: Sized;
}

pub fn read_delta_instruction_from(
    stream: &mut dyn Read,
) -> Result<Option<DeltaInstruction>, CommandError> {
    let mut first_byte = [0; 1];
    if stream.read_exact(&mut first_byte).is_err() {
        return Ok(None);
    }
    let byte = first_byte[0];
    if byte & 0b1000_0000 == 0 {
        Ok(Some(DataInstruction::read_from(byte, stream)?))
    } else {
        Ok(Some(CopyInstruction::read_from(byte, stream)?))
    }
}
