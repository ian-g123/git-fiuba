use super::delta_instruction::{DeltaInstruction, DeltaInstructionTrait};
use crate::command_errors::CommandError;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct DataInstruction {
    data_to_append: Vec<u8>,
}

impl DeltaInstructionTrait for DataInstruction {
    fn apply(&self, new_object: &mut dyn Write, _: &[u8]) -> Result<(), CommandError> {
        new_object.write_all(&self.data_to_append).map_err(|_| {
            crate::command_errors::CommandError::FileWriteError(
                "Error escribiendo instruccion delta".to_string(),
            )
        })
    }

    fn read_from(first_byte: u8, stream: &mut dyn Read) -> Result<DeltaInstruction, CommandError>
    where
        Self: Sized,
    {
        let bytes_to_read = first_byte & 0b0111_1111;

        let bytes_to_read = bytes_to_read as usize;
        let mut data_to_append = vec![0; bytes_to_read];
        stream.read_exact(&mut data_to_append).map_err(|_| {
            crate::command_errors::CommandError::FileWriteError(
                "Error escribiendo instruccion delta".to_string(),
            )
        })?;
        Ok(Box::new(DataInstruction { data_to_append }))
    }
}
