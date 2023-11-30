use crate::command_errors::CommandError;

#[derive(Debug, PartialEq, Clone)]
pub enum MergeStage {
    RegularFile = 0, // ver si se tiene en cuenta
    Common = 1,
    Head = 2,
    Destin = 3,
}

impl MergeStage {
    pub fn from_u8(value: u8) -> Result<MergeStage, CommandError> {
        match value {
            0 => Ok(MergeStage::RegularFile),
            1 => Ok(MergeStage::Common),
            2 => Ok(MergeStage::Head),
            3 => Ok(MergeStage::Destin),
            _ => Err(CommandError::FileReadError(
                "Tipo de merge stage inválido".to_string(),
            )),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::RegularFile => 0,
            Self::Common => 1,
            Self::Head => 2,
            Self::Destin => 3,
        }
    }
}
