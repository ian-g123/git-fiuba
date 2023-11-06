use std::fmt;

use crate::command_errors::CommandError;

#[derive(Debug)]
pub enum PackfileObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
    OfsDelta,
    RefDelta,
}

impl PackfileObjectType {
    pub fn from_u8(byte: u8) -> Result<Self, CommandError> {
        match byte {
            1 => Ok(PackfileObjectType::Commit),
            2 => Ok(PackfileObjectType::Tree),
            3 => Ok(PackfileObjectType::Blob),
            4 => Ok(PackfileObjectType::Tag),
            6 => Ok(PackfileObjectType::OfsDelta),
            7 => Ok(PackfileObjectType::RefDelta),
            _ => Err(CommandError::UnknownObjectType),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            PackfileObjectType::Commit => 1,
            PackfileObjectType::Tree => 2,
            PackfileObjectType::Blob => 3,
            PackfileObjectType::Tag => 4,
            PackfileObjectType::OfsDelta => 6,
            PackfileObjectType::RefDelta => 7,
        }
    }

    pub fn from_str(name: &str) -> Result<Self, CommandError> {
        match name {
            "commit" => Ok(PackfileObjectType::Commit),
            "tree" => Ok(PackfileObjectType::Tree),
            "blob" => Ok(PackfileObjectType::Blob),
            "tag" => Ok(PackfileObjectType::Tag),
            _ => Err(CommandError::UnknownObjectType),
        }
    }
}

impl fmt::Display for PackfileObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackfileObjectType::Commit => write!(f, "commit"),
            PackfileObjectType::Tree => write!(f, "tree"),
            PackfileObjectType::Blob => write!(f, "blob"),
            PackfileObjectType::Tag => write!(f, "tag"),
            PackfileObjectType::OfsDelta => write!(f, "ofs-delta"),
            PackfileObjectType::RefDelta => write!(f, "ref-delta"),
        }
    }
}
