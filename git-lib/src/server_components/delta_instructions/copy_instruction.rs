use std::io::{Read, Write};

use crate::command_errors::CommandError;

use super::delta_instruction::{DeltaInstruction, DeltaInstructionTrait};

#[derive(Debug)]
pub struct CopyInstruction {
    offset: u32,
    size: u32,
}

impl DeltaInstructionTrait for CopyInstruction {
    fn apply(&self, new_object: &mut dyn Write, base_object: &[u8]) -> Result<(), CommandError> {
        new_object
            .write_all(&base_object[self.offset as usize..(self.offset + self.size) as usize])
            .map_err(|_| {
                crate::command_errors::CommandError::FileWriteError(
                    "Error escribiendo instruccion delta".to_string(),
                )
            })
    }

    fn read_from(first_byte: u8, stream: &mut dyn Read) -> Result<DeltaInstruction, CommandError>
    where
        Self: Sized,
    {
        if first_byte == 0b1000_0000 {
            return Ok(Box::new(Self {
                offset: 0,
                size: 0x10000,
            }));
        }

        let mut offset = 0;
        for i in 0..4 {
            if first_byte >> i & 1 == 0 {
                continue;
            }
            let mut byte = [0; 1];
            stream.read_exact(&mut byte).map_err(|_| {
                crate::command_errors::CommandError::FileWriteError(
                    "Error escribiendo instruccion delta".to_string(),
                )
            })?;

            offset |= (byte[0] as u32) << (i * 8);
        }

        let mut size = 0;
        for i in 0..3 {
            if first_byte >> (i + 4) & 1 == 0 {
                continue;
            }
            let mut byte = [0; 1];
            stream.read_exact(&mut byte).map_err(|_| {
                crate::command_errors::CommandError::FileWriteError(
                    "Error escribiendo instruccion delta".to_string(),
                )
            })?;
            size |= (byte[0] as u32) << (i * 8);
        }

        if size == 0 {
            size = 0x10000;
        }
        Ok(Box::new(Self { offset, size }))
    }
}
