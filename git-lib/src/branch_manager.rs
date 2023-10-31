use crate::command_errors::CommandError;
use std::{fs::File, io::Read};

// /// Obtiene la ruta de la rama actual.
// pub fn get_head_branch() -> Result<String, CommandError> {
//     let mut branch = String::new();
//     let path = ".git/HEAD";
//     let Ok(mut head) = File::open(path) else {
//         return Err(CommandError::NotGitRepository);
//     };

//     if head.read_to_string(&mut branch).is_err() {
//         return Err(CommandError::FileReadError(path.to_string()));
//     }

//     let branch = branch.trim();
//     let Some(branch) = branch.split(" ").last() else {
//         return Err(CommandError::HeadError);
//     };
//     Ok(branch.to_string())
// }

/* pub fn get_current_branch_name() -> Result<String, CommandError> {
    let branch = get_head_branch()?;
    let branch_name: Vec<&str> = branch.split_terminator("/").collect();
    Ok(branch_name[branch_name.len() - 1].to_string())
} */

// Obtiene el hash del Commit padre.
