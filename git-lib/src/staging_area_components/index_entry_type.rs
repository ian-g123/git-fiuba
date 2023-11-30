use crate::command_errors::CommandError;

#[derive(Debug, PartialEq, Clone)]
pub enum IndexEntryType {
    RegularFile = 1000,
    SymLink = 1010,
    GitLink = 1110,
}

impl IndexEntryType {
    pub fn from_u8(value: u8) -> Result<IndexEntryType, CommandError> {
        match value {
            0b00001000 => Ok(IndexEntryType::RegularFile),
            0b00001010 => Ok(IndexEntryType::SymLink),
            0b00001110 => Ok(IndexEntryType::GitLink),
            _ => Err(CommandError::FileReadError(
                "Tipo de index entry inválido".to_string(),
            )),
        }
    }
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::RegularFile => 0b00001000,
            Self::SymLink => 0b00001010,
            Self::GitLink => 0b00001110,
        }
    }
}
