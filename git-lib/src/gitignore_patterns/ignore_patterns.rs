use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    command_errors::CommandError,
    git_repository::{get_path_str, next_line},
    join_paths,
    logger::Logger,
};

use super::pattern::Pattern;

const BACKSLASH: char = '\\';
const SLASH: char = '/';
const ASTERISK: char = '*';
const NEGATE: char = '!';
const RANGE: char = '[';
const ONE_CHAR: char = '?';

#[derive(Clone)]
pub struct GitignorePatterns {
    patterns: Vec<(String, Vec<(usize, Pattern)>)>,
}

impl GitignorePatterns {
    /// Crea un GitignorePatterns que lee todos los archivos .gitignore del repositorio y .git/info/exclude
    /// y guarda los patrones establecidos en ellos.
    pub fn new(
        git_path: &str,
        working_dir_path: &str,
        logger: &mut Logger,
    ) -> Result<Self, CommandError> {
        let mut gitignore_files = get_gitignore_files(git_path, working_dir_path, logger)?;

        let working_dir = format!("./{}", working_dir_path);
        look_for_gitignore_files(&working_dir, &mut gitignore_files, logger, &working_dir)?;
        let mut patterns: Vec<(String, Vec<(usize, Pattern)>)> = Vec::new();
        for gitignore_path in gitignore_files.iter() {
            add_gitignore_patterns(gitignore_path, &mut patterns, logger)?;
        }
        Ok(Self { patterns })
    }

    /// Dado un path, determina si el mismo debe ser ignorado por git según los patrones guardados por
    /// la estructura.
    pub fn must_be_ignored(
        &self,
        path: &str,
        logger: &mut Logger,
    ) -> Result<Option<(String, usize, Pattern)>, CommandError> {
        let base_path = env::current_dir()
            .map_err(|_| CommandError::FileNotFound("Directorio actual".to_string()))?;
        let path = if path.starts_with("../") || path.starts_with("./") {
            let Some(path) = get_real_path(path, &base_path) else {
                return Err(CommandError::OutsideOfRepository(
                    path.to_string(),
                    base_path,
                ));
            };
            path
        } else {
            path.to_string()
        };

        let mut negate_pattern: Option<(String, usize, Pattern)> = None;
        let mut last_pattern: Option<(String, usize, Pattern)> = None;

        for (dir_level, pattern_vec) in self.patterns.iter() {
            for (line_number, pattern) in pattern_vec {
                let base_path = if dir_level.ends_with(".gitignore") {
                    dir_level[..dir_level.len() - 10].to_string()
                } else {
                    dir_level[..dir_level.len() - 17].to_string()
                };

                if matches_pattern(&path, pattern, &base_path, logger)? {
                    logger.log("matches pattern");
                    if pattern.negate_pattern() {
                        negate_pattern = Some((
                            dir_level.to_string(),
                            line_number.to_owned(),
                            pattern.clone(),
                        ));
                    } else {
                        last_pattern = Some((
                            dir_level.to_string(),
                            line_number.to_owned(),
                            pattern.clone(),
                        ));
                    }
                }
            }
        }
        if negate_pattern.is_some() {
            return Ok(negate_pattern);
        }
        Ok(last_pattern)
    }
}

/// Obtiene la lista de archivos gitignore. Agrega .git/info/exclude.
fn get_gitignore_files(
    git_path: &str,
    working_dir_path: &str,
    _logger: &mut Logger,
) -> Result<Vec<String>, CommandError> {
    let mut gitignore_base_path = working_dir_path;
    let mut gitignore_files: Vec<String> = Vec::new();
    if let Some(working_dir_repo) = git_path.strip_suffix(".git") {
        while working_dir_repo != gitignore_base_path {
            let gitignore_path = join_paths!(gitignore_base_path, ".gitignore").ok_or(
                CommandError::FileCreationError(format!(
                    " No se pudo formar la ruta del archivo gitignore"
                )),
            )?;
            if Path::new(&gitignore_path).exists() {
                gitignore_files.insert(0, gitignore_path);
            }
            let Some((new_base_path, _)) = gitignore_base_path.split_once("/") else {
                break;
            };
            gitignore_base_path = new_base_path;
        }
        let exclude_path =
            join_paths!(git_path, "info/exclude").ok_or(CommandError::FileCreationError(
                format!(" No se pudo formar la ruta del archivo info/exclude"),
            ))?;
        if Path::new(&exclude_path).exists() {
            gitignore_files.insert(0, exclude_path);
        }
    }

    Ok(gitignore_files)
}

/// Lee un archivo de gitignore o exclude y obtiene los patrones del mismo.
fn add_gitignore_patterns(
    gitignore_path: &str,
    patterns: &mut Vec<(String, Vec<(usize, Pattern)>)>,
    _logger: &mut Logger,
) -> Result<(), CommandError> {
    let mut patterns_hashmap: Vec<(usize, Pattern)> = Vec::new();
    let content = fs::read_to_string(gitignore_path)
        .map_err(|error| CommandError::FileReadError(error.to_string()))?;

    let mut lines = content.lines();
    let mut line_number = 0;

    loop {
        let (eof, line) = next_line(&mut lines);
        if eof {
            break;
        }

        if line.starts_with("#") || line == "".to_string() {
            line_number += 1;
            continue;
        }

        let line = line.trim().to_string();
        let mut is_relative = false;
        let mut path = String::new();
        let pattern: Pattern;
        let mut ignore_pattern = false;
        let mut negate_pattern = false;
        let mut starts_with = false;
        let mut ends_with = false;

        let last_index = line.len() - 1;
        for (i, character) in line.char_indices() {
            if ignore_pattern {
                path += &character.to_string();
                continue;
            }
            match character {
                NEGATE => negate_pattern = true,

                BACKSLASH => ignore_pattern = true,
                ASTERISK => {
                    if i == 0 || (i == 0 && (line.starts_with("/") || line.starts_with("!"))) {
                        ends_with = true;
                    } else if i == last_index {
                        starts_with = true;
                    } else {
                        return Err(CommandError::FeatureNotImplemented(line));
                    }
                    if i == last_index - 1 && line[last_index..].starts_with(ASTERISK) {
                        return Err(CommandError::FeatureNotImplemented("**".to_string()));
                    }
                }
                SLASH => {
                    if i != last_index {
                        if !(i == last_index - 1 && line[last_index..].starts_with(ASTERISK)) {
                            is_relative = true;
                        }
                    }
                    if i != 0 {
                        path += &character.to_string();
                    }
                }
                RANGE => return Err(CommandError::FeatureNotImplemented(RANGE.to_string())),
                ONE_CHAR => return Err(CommandError::FeatureNotImplemented(ONE_CHAR.to_string())),

                _ => path += &character.to_string(),
            }
        }

        if starts_with {
            pattern = Pattern::StartsWith(line, path, is_relative, negate_pattern);
        } else if ends_with {
            pattern = Pattern::EndsWith(line, path, is_relative, negate_pattern);
        } else if is_relative {
            pattern = Pattern::RelativeToDirLevel(line, path, negate_pattern);
        } else {
            pattern = Pattern::NotRelativeToDirLevel(line, path, negate_pattern);
        }

        patterns_hashmap.push((line_number + 1, pattern));
        line_number += 1;
    }
    _ = patterns.push((gitignore_path.to_string(), patterns_hashmap));
    Ok(())
}

/// Dado un path que puede empezar con . o ../, obtiene el path real relativo al directorio actual.
fn get_real_path(target_path: &str, base_path: &PathBuf) -> Option<String> {
    if let Ok(absolute_path) = fs::canonicalize(target_path) {
        if let Ok(relative_path) = absolute_path.strip_prefix(base_path) {
            if let Some(real_path_str) = relative_path.to_str() {
                return Some(real_path_str.to_string());
            }
        }
    }
    None
}

/// Devuelve true si el path coincide con el patrón.
fn matches_pattern(
    path: &str,
    pattern: &Pattern,
    _base_path: &str,
    logger: &mut Logger,
) -> Result<bool, CommandError> {
    match pattern {
        Pattern::StartsWith(_, pattern, is_relative, _) => {
            logger.log(&format!(
                "STARTS WITH --> path: {}, pattern: {}, is_relative: {}, is_dir: {}",
                path,
                pattern,
                is_relative,
                is_dir(pattern)
            ));

            return matches_starts_with(path, pattern, is_relative.to_owned());
        }
        Pattern::EndsWith(_, pattern, is_relative, _) => {
            logger.log(&format!(
                "ENDS WITH --> path: {}, pattern: {}, is_dir: {}, is_relative: {}",
                path,
                pattern,
                is_dir(pattern),
                is_relative.to_owned()
            ));

            return matches_ends_with(path, pattern, is_relative.to_owned());
        }

        Pattern::RelativeToDirLevel(_, pattern, _) => {
            logger.log(&format!(
                "RELATIVE --> path: {}, pattern: {}",
                path, pattern
            ));
            return matches_relative_pattern(path, pattern);
        }
        Pattern::NotRelativeToDirLevel(_, pattern, _) => {
            logger.log(&format!(
                "NOT RELATIVE--> path: {}, pattern: {}",
                path, pattern
            ));

            return matches_non_relative_pattern(path, pattern);
        }
    }
}

/// Busca desde 'path_name' archivos .gitignore.
fn look_for_gitignore_files(
    path_name: &str,
    gitignore_files: &mut Vec<String>,
    logger: &mut Logger,
    base_path: &str,
) -> Result<(), CommandError> {
    let path = Path::new(path_name);

    let Ok(entries) = fs::read_dir(path.clone()) else {
        return Err(CommandError::DirNotFound(path_name.to_owned()));
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return Err(CommandError::DirNotFound(path_name.to_owned()));
        };
        let entry_path = entry.path();
        let entry_name = get_path_str(entry_path.clone())?;

        if entry_name.contains(".git/") {
            continue;
        }
        if entry_path.is_dir() {
            look_for_gitignore_files(&entry_name, gitignore_files, logger, base_path)?;
        } else if entry_name.ends_with(".gitignore") {
            if let Some(path) = entry_name.strip_prefix(base_path) {
                let path = if path.starts_with("/") {
                    path[1..].to_string()
                } else {
                    path.to_string()
                };
                if !gitignore_files.contains(&path) {
                    gitignore_files.push(path);
                }
            }
        }
    }
    Ok(())
}

/// Devuelve true si el path pasado es un directorio.
fn is_dir(path: &str) -> bool {
    let obj_path = Path::new(path);
    path.ends_with("/") || obj_path.is_dir()
}

/// Devuelve true si el path coincide con el patrón STARTS_WITH
fn matches_starts_with(path: &str, pattern: &str, is_relative: bool) -> Result<bool, CommandError> {
    if is_relative || is_dir(pattern) {
        if path.starts_with(pattern) {
            return Ok(true);
        }
        return Ok(false);
    }
    let mut index = 0;

    if !is_relative || !is_dir(pattern) {
        for _ in path.chars() {
            if path[index..].starts_with(pattern) {
                if index > 0 && !path[index - 1..].starts_with("/") {
                    return Ok(false);
                }
                return Ok(true);
            }
            index += 1;
        }
    }
    Ok(false)
}

/// Devuelve true si el path coincide con el patrón ENDS_WITH
fn matches_ends_with(path: &str, pattern: &str, is_relative: bool) -> Result<bool, CommandError> {
    let mut s = 0;
    let mut index = 0;

    if path.contains(pattern) {
        for _ in path.chars() {
            if path[index..].starts_with(pattern) {
                s = index;
                break;
            }
            index += 1;
        }
        let e = s + pattern.len();

        if (e < path.len() - 1 && !path[e..].starts_with("/") && !is_dir(pattern))
            || (is_relative.to_owned() && s > 0 && path[..s].contains("/"))
        {
            return Ok(false);
        }
        return Ok(true);
    }
    Ok(false)
}

/// Devuelve true si el path coincide con el patrón RELATIVE_TO_DIR_LEVEL
fn matches_relative_pattern(path: &str, pattern: &str) -> Result<bool, CommandError> {
    if path.starts_with(pattern) {
        if let Some(rest) = path.get(pattern.len()..) {
            if rest != "" && !rest.starts_with("/") && !is_dir(pattern) {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Ok(false)
}

/// Devuelve true si el path coincide con el patrón NOT_RELATIVE_TO_DIR_LEVEL
fn matches_non_relative_pattern(path: &str, pattern: &str) -> Result<bool, CommandError> {
    let mut index = 0;

    if path.contains(pattern) {
        for _ in path.chars() {
            if path[index..].starts_with(pattern) {
                if let Some(rest) = path.get(index + pattern.len()..) {
                    if rest != "" && !rest.starts_with("/") && !is_dir(pattern) {
                        return Ok(false);
                    }
                }
                if let Some(rest) = path.get(..index) {
                    if rest != "" && !rest.ends_with("/") {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            index += 1;
        }
    }
    Ok(false)
}
