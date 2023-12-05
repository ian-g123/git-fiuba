use std::{
    io::{Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
};

use crate::{
    command_errors::CommandError,
    utils::{aux::read_string_until, super_string::SuperStrings},
};

use super::{index_entry_type::IndexEntryType, merge_stage::MergeStage};

#[derive(Debug, Clone, PartialEq)]
pub struct IndexEntry {
    pub ctime: (i32, i32), // ctime (seconds, nanosecond), the last time a file's metadata changed

    pub mtime: (i32, i32), // mtime (seconds, nanosecond), the last time a file's data changed

    pub dev: u32,                   // dev
    pub ino: u32,                   // ino
    pub entry_type: IndexEntryType, // 4-bit object type (1000 (regular file), 1010 (symbolic link) and 1110 (gitlink))
    pub unix_permission: u16, // 9-bit unix permission. Only 0755 and 0644 are valid for regular files. Symbolic links and gitlinks have value 0 in this field.
    pub uid: u32,             // uid
    pub gid: u32,             // gid
    pub fsize: u32, // file size This is the on-disk size from stat(2), truncated to 32-bit.
    pub sha1: [u8; 20], // Object name for the represented object (160-bit SHA-1 Object ID)
    // A 16-bit 'flags' field split into (high to low bits)
    // hex: 0009
    // bin: 00000000 00001001
    pub assume_valid: bool, // 1-bit assume-valid flag
    pub stage: MergeStage, // 2-bit stage (during merge) --> Reg file = 0, common = 1, head = 2, destin = 3
                           // Entry path name (variable length) relative to top level directory (without leading slash).
                           // '/' is used as path separator. The special
                           //   path components ".", ".." and ".git" (without quotes) are disallowed.
                           //   Trailing slash is also disallowed.
                           // The exact encoding is undefined, but the '.' and '/' characters
                           // are encoded in 7-bit ASCII and the encoding cannot contain a NUL
                           // byte (iow, this is a UNIX pathname).
                           // (Version 4) In version 4, the entry path name is prefix-compressed
                           //   relative to the path name for the previous entry (the very first
                           //   entry is encoded as if the path name for the previous entry is an
                           //   empty string).  At the beginning of an entry, an integer N in the
                           //   variable width encoding (the same encoding as the offset is encoded
                           //   for OFS_DELTA pack entries; see pack-format.txt) is stored, followed
                           //   by a NUL-terminated string S.  Removing N bytes from the end of the
                           //   path name for the previous entry, and replacing it with the string S
                           //   yields the path name for this entry.
                           // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
                           // while keeping the name NUL-terminated.
                           // (Version 4) In version 4, the padding after the pathname does not
                           // exist.
                           // Interpretation of index entries in split index mode is completely
                           // different. See below for details.
}

impl IndexEntry {
    pub fn read_from_stream(stream: &mut dyn Read) -> Result<(String, Self), CommandError> {
        let mut ctime_seconds_bytes = [0; 4];
        stream
            .read_exact(&mut ctime_seconds_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let mut ctime_nanoseconds_bytes = [0; 4];
        stream
            .read_exact(&mut ctime_nanoseconds_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let ctime = (
            i32::from_be_bytes(ctime_seconds_bytes),
            i32::from_be_bytes(ctime_nanoseconds_bytes),
        );
        let mut mtime_seconds_bytes = [0; 4];
        stream
            .read_exact(&mut mtime_seconds_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let mut mtime_nanoseconds_bytes = [0; 4];
        stream
            .read_exact(&mut mtime_nanoseconds_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let mtime = (
            i32::from_be_bytes(mtime_seconds_bytes),
            i32::from_be_bytes(mtime_nanoseconds_bytes),
        );
        let mut dev_bytes = [0; 4];
        stream
            .read_exact(&mut dev_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let dev = u32::from_be_bytes(dev_bytes);
        let mut ino_bytes = [0; 4];
        stream
            .read_exact(&mut ino_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let ino = u32::from_be_bytes(ino_bytes);
        let mut empty_bytes = [0; 2];
        stream
            .read_exact(&mut empty_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        if empty_bytes != [0; 2] {
            return Err(CommandError::FileReadError(
                "Empty bytes no son 0".to_string(),
            ));
        }
        let mut type_and_permission_bytes = [0; 2];
        stream
            .read_exact(&mut type_and_permission_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let type_value = u8::from_be_bytes([type_and_permission_bytes[0] >> 4]);
        let entry_type = IndexEntryType::from_u8(type_value)?;
        let unix_permission = u16::from_be_bytes([
            type_and_permission_bytes[0] & 0b00000001,
            type_and_permission_bytes[1],
        ]);
        let mut uid_bytes: [u8; 4] = [0; 4];
        stream
            .read_exact(&mut uid_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let uid = u32::from_be_bytes(uid_bytes);
        let mut gid_bytes = [0; 4];
        stream
            .read_exact(&mut gid_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let gid = u32::from_be_bytes(gid_bytes);
        let mut fsize_bytes = [0; 4];
        stream
            .read_exact(&mut fsize_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let fsize = u32::from_be_bytes(fsize_bytes);
        let mut sha1 = [0; 20];
        stream
            .read_exact(&mut sha1)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let mut flags_bytes = [0; 2];
        stream
            .read_exact(&mut flags_bytes)
            .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        let assume_valid = flags_bytes[0] & 0b10000000 != 0;
        let extended = flags_bytes[0] & 0b01000000 != 0;
        if extended {
            return Err(CommandError::FileReadError(
                "No se soporta el formato no extendido".to_string(),
            ));
        }
        let stage = MergeStage::from_u8((flags_bytes[0] & 0b00110000) >> 4)?;
        let path_length = match u16::from_be_bytes([flags_bytes[0] & 0b00001111, flags_bytes[1]]) {
            0xFFF => None,
            length => Some(length),
        };
        let mut bytes_count = 62;
        let path = match path_length {
            Some(length) => {
                let mut path_bytes = vec![0; length as usize];
                stream
                    .read_exact(&mut path_bytes)
                    .map_err(|error| CommandError::FileReadError(error.to_string()))?;
                let mut null_byte = vec![0; 1];
                stream
                    .read_exact(&mut null_byte)
                    .map_err(|error| CommandError::FileReadError(error.to_string()))?;
                if null_byte[0] != 0 {
                    return Err(CommandError::FileReadError("Null byte no es 0".to_string()));
                }
                bytes_count += length as u32 + 1;
                String::from_utf8(path_bytes).map_err(|error| {
                    CommandError::FileReadError(format!(
                        "Error convirtiendo path a string{}",
                        error
                    ))
                })?
            }
            None => {
                let path_str = read_string_until(stream, '\0')?;
                bytes_count += path_str.len() as u32 + 1;
                path_str
            }
        };
        let padding = 8 - (bytes_count % 8);
        if padding != 8 {
            let mut padding_bytes = vec![0; padding as usize];
            stream
                .read_exact(&mut padding_bytes)
                .map_err(|error| CommandError::FileReadError(error.to_string()))?;
        }
        let entry = IndexEntry {
            ctime,
            mtime,
            dev,
            ino,
            entry_type,
            unix_permission,
            uid,
            gid,
            fsize,
            sha1,
            assume_valid,
            stage,
        };
        Ok((path, entry))
    }

    pub fn new(metadata: &std::fs::Metadata, hash: &str) -> Result<IndexEntry, CommandError> {
        let ctime = metadata
            .created()
            .map_err(|error| {
                CommandError::MetadataError(format!(
                    "No se pudo obtener el ctime del archivo: {}",
                    error
                ))
            })?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| {
                CommandError::MetadataError(format!(
                    "No se pudo obtener el ctime del archivo: {}",
                    error
                ))
            })?;
        let mtime = metadata
            .modified()
            .map_err(|error| {
                CommandError::MetadataError(format!(
                    "No se pudo obtener el mtime del archivo: {}",
                    error
                ))
            })?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| {
                CommandError::MetadataError(format!(
                    "No se pudo obtener el mtime del archivo: {}",
                    error
                ))
            })?;
        let dev = metadata.dev() as u32;
        let ino = metadata.ino() as u32;

        let entry_type = IndexEntryType::RegularFile;
        let unix_permission = metadata.permissions().mode() as u16;
        let uid = metadata.uid();
        let gid = metadata.gid();
        let fsize = metadata.len() as u32;

        let sha1 = hash.to_string().cast_hex_to_u8_vec()?;
        let assume_valid = false;
        let stage = MergeStage::RegularFile;
        Ok(IndexEntry {
            ctime: (ctime.as_secs() as i32, ctime.subsec_nanos() as i32),
            mtime: (mtime.as_secs() as i32, mtime.subsec_nanos() as i32),
            dev,
            ino,
            entry_type,
            unix_permission,
            uid,
            gid,
            fsize,
            sha1,
            assume_valid,
            stage,
        })
    }

    pub fn new_conflicting(
        metadata: &std::fs::Metadata,
        hash: &str,
        stage: MergeStage,
    ) -> Result<IndexEntry, CommandError> {
        let unix_permission = metadata.permissions().mode() as u16;

        let sha1 = hash.to_string().cast_hex_to_u8_vec()?;

        Ok(IndexEntry {
            ctime: (0, 0),
            mtime: (0, 0),
            dev: 0,
            ino: 0,
            entry_type: IndexEntryType::RegularFile,
            unix_permission,
            uid: 0,
            gid: 0,
            fsize: 0,
            sha1,
            assume_valid: false,
            stage,
        })
    }

    pub fn write_to(&self, stream: &mut dyn Write, path: &str) -> Result<(), CommandError> {
        let ctime_seconds_bytes = self.ctime.0.to_be_bytes();
        stream
            .write_all(&ctime_seconds_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let ctime_nanoseconds_bytes = self.ctime.1.to_be_bytes();
        stream
            .write_all(&ctime_nanoseconds_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let mtime_seconds_bytes = self.mtime.0.to_be_bytes();
        stream
            .write_all(&mtime_seconds_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let mtime_nanoseconds_bytes = self.mtime.1.to_be_bytes();
        stream
            .write_all(&mtime_nanoseconds_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let dev_bytes = self.dev.to_be_bytes();
        stream
            .write_all(&dev_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let ino_bytes = self.ino.to_be_bytes();
        stream
            .write_all(&ino_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let empty_bytes = [0; 2];
        stream
            .write_all(&empty_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let type_and_permission_bytes = [
            (self.entry_type.to_u8() << 4) | self.unix_permission.to_be_bytes()[0],
            self.unix_permission.to_be_bytes()[1],
        ];
        stream
            .write_all(&type_and_permission_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let uid_bytes = self.uid.to_be_bytes();
        stream
            .write_all(&uid_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let gid_bytes = self.gid.to_be_bytes();
        stream
            .write_all(&gid_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let fsize_bytes = self.fsize.to_be_bytes();
        stream
            .write_all(&fsize_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        stream
            .write_all(&self.sha1)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;

        let name_length = path.len();
        let name_length_bytes = match name_length {
            0xFFF => [0x0F, 0xFF],
            value => (value as u16).to_be_bytes(),
        };
        let flags_bytes = [
            if self.assume_valid {
                0b10000000
            } else {
                0b00000000
            } | self.stage.to_u8() << 4
                | name_length_bytes[0],
            name_length_bytes[1],
        ];
        stream
            .write_all(&flags_bytes)
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        stream
            .write_all(path.as_bytes())
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        stream
            .write_all(&[0])
            .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        let padding = 8 - ((62 + path.len() + 1) % 8);
        if padding != 8 {
            let padding_bytes = vec![0; padding];
            stream
                .write_all(&padding_bytes)
                .map_err(|error| CommandError::FileWriteError(error.to_string()))?;
        }
        Ok(())
    }
}
