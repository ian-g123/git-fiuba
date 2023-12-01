use std::{collections::HashMap, fs, path::Path};

use crate::{command_errors::CommandError, git_repository::next_line, join_paths};

const BACKSLASH: char = '/';
const SLASH: char = '\\';
const ASTERISK: char = '*';
const NEGATE: char = '!';

pub struct GitignorePatterns {
    patterns: Vec<(String, Vec<(usize, Pattern)>)>,
}

impl GitignorePatterns {
    pub fn new(git_path: &str, working_dir_path: &str) -> Result<Self, CommandError> {
        let gitignore_files = get_gitignore_files(git_path, working_dir_path)?;
        let mut patterns: Vec<(String, Vec<(usize, Pattern)>)> = Vec::new();
        for gitignore_path in gitignore_files.iter() {
            add_gitignore_patterns(gitignore_path, &mut patterns)?;
        }
        Ok(Self { patterns })
    }

    pub fn must_be_ignored(
        &self,
        path: &str,
    ) -> Result<Option<(String, usize, Pattern)>, CommandError> {
        let Some(path) = get_real_path(path) else {
            return Ok(None);
        };
        let mut negate_pattern: Option<(String, usize, Pattern)> = None;
        let mut patterns_matched: Vec<(String, usize, Pattern)> = Vec::new();
        for (dir_level, pattern_hashmap) in self.patterns.iter() {
            for (line_number, pattern) in pattern_hashmap {
                if matches_pattern(&path, pattern, &dir_level[..dir_level.len() - 10])? {
                    patterns_matched.push((
                        dir_level.to_string(),
                        line_number.to_owned(),
                        pattern.to_owned(),
                    ));
                    if pattern.negate_pattern() {
                        negate_pattern = Some((
                            dir_level.to_string(),
                            line_number.to_owned(),
                            pattern.clone(),
                        ));
                    } else if negate_pattern.is_some() {
                        negate_pattern = None;
                    }
                }
            }
        }
        if negate_pattern.is_some() {
            return Ok(negate_pattern);
        } else if !patterns_matched.is_empty() {
            return Ok(Some(patterns_matched[0].clone()));
        }
        Ok(None)
    }
}

fn get_gitignore_files(
    git_path: &str,
    working_dir_path: &str,
) -> Result<Vec<String>, CommandError> {
    let mut gitignore_base_path = working_dir_path;
    let mut gitignore_files: Vec<String> = Vec::new();
    if let Some((working_dir_repo, _)) = git_path.split_once("/") {
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
        let gitignore_path =
            join_paths!(git_path, "info/exclude").ok_or(CommandError::FileCreationError(
                format!(" No se pudo formar la ruta del archivo info/exclude"),
            ))?;
        if Path::new(&gitignore_path).exists() {
            gitignore_files.insert(0, gitignore_path);
        }
    }

    Ok(gitignore_files)
}

fn add_gitignore_patterns(
    gitignore_path: &str,
    patterns: &mut Vec<(String, Vec<(usize, Pattern)>)>,
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
        let mut matches = false;
        let mut asterisk_index = 0;

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
                        matches = true;
                        asterisk_index = i;
                    }
                }
                SLASH => {
                    if i != last_index {
                        is_relative = true;
                    }
                    if i != 0 {
                        path += &character.to_string();
                    }
                }
                _ => path += &character.to_string(),
            }
        }

        if starts_with {
            pattern = Pattern::StartsWith(line, path, is_relative, negate_pattern);
        } else if ends_with {
            pattern = Pattern::EndsWith(line, path, is_relative, negate_pattern);
        } else if matches {
            pattern = Pattern::MatchesAsterisk(
                line,
                path[..asterisk_index].to_string(),
                path[asterisk_index..].to_string(),
                is_relative,
                negate_pattern,
            );
        } else if is_relative {
            pattern = Pattern::RelativeToDirLevel(line, path, negate_pattern);
        } else {
            pattern = Pattern::NotRelativeToDirLevel(line, path, negate_pattern);
        }

        patterns_hashmap.push((line_number, pattern));
        line_number += 1;
    }
    _ = patterns.push((gitignore_path.to_string(), patterns_hashmap));
    Ok(())
}

#[derive(Clone)]
pub enum Pattern {
    StartsWith(String, String, bool, bool),
    EndsWith(String, String, bool, bool),
    RelativeToDirLevel(String, String, bool),
    NotRelativeToDirLevel(String, String, bool),
    MatchesAsterisk(String, String, String, bool, bool),
    // si queda tiempo, agregar: ? (MatchesOne), [a-z] (MatchesRange), **
}

impl Pattern {
    fn negate_pattern(&self) -> bool {
        match self {
            Self::StartsWith(_, _, _, negate) => negate.to_owned(),
            Self::EndsWith(_, _, _, negate) => negate.to_owned(),
            Self::MatchesAsterisk(_, _, _, _, negate) => negate.to_owned(),
            Self::RelativeToDirLevel(_, _, negate) => negate.to_owned(),
            Self::NotRelativeToDirLevel(_, _, negate) => negate.to_owned(),
        }
    }

    fn is_relative(&self) -> bool {
        match self {
            Self::StartsWith(_, _, is_relative, _) => is_relative.to_owned(),
            Self::EndsWith(_, _, is_relative, _) => is_relative.to_owned(),
            Self::MatchesAsterisk(_, _, _, is_relative, _) => is_relative.to_owned(),
            Self::RelativeToDirLevel(_, _, _) => true,
            Self::NotRelativeToDirLevel(__, _, _) => false,
        }
    }

    fn get_pattern_read(&self) -> String {
        match self {
            Self::StartsWith(pattern_extracted, _, _, _) => pattern_extracted.to_string(),

            Self::EndsWith(pattern_extracted, _, _, _) => pattern_extracted.to_string(),
            Self::MatchesAsterisk(pattern_extracted, _, _, _, _) => pattern_extracted.to_string(),
            Self::RelativeToDirLevel(pattern_extracted, _, _) => pattern_extracted.to_string(),
            Self::NotRelativeToDirLevel(pattern_extracted, _, _) => pattern_extracted.to_string(),
        }
    }

    pub fn to_string(
        &self,
        path: &str,
        gitignore_path: &str,
        line_number: usize,
        verbose: bool,
    ) -> String {
        if verbose {
            return format!(
                "{}:{}:{}\t{}\n",
                gitignore_path,
                line_number,
                self.get_pattern_read(),
                path
            );
        }
        format!("{}\n", path)
    }
}

fn get_real_path(relative_path: &str) -> Option<String> {
    if let Ok(real_path) = fs::canonicalize(relative_path) {
        if let Some(real_path_str) = real_path.to_str() {
            return Some(real_path_str.to_string());
        }
    }
    None
}

fn matches_pattern(path: &str, pattern: &Pattern, base_path: &str) -> Result<bool, CommandError> {
    let path = {
        if pattern.is_relative() {
            path.to_string()
        } else {
            join_paths!(base_path, path).ok_or(CommandError::FileCreationError(format!(
                " No se pudo formar la ruta del archivo"
            )))?
        }
    };
    match pattern {
        Pattern::StartsWith(_, pattern, _, _) => {
            if path.starts_with(pattern) {
                return Ok(true);
            }
        }
        Pattern::EndsWith(_, pattern, _, _) => {
            if path.ends_with(pattern) {
                return Ok(true);
            }
        }
        Pattern::MatchesAsterisk(_, start, end, _, _) => {
            if path.starts_with(start) && path.ends_with(end) {
                return Ok(true);
            }
        }
        Pattern::RelativeToDirLevel(_, pattern, _)
        | Pattern::NotRelativeToDirLevel(_, pattern, _) => {
            let mut path = Path::new(&path);
            while let Some(parent) = path.parent() {
                if parent.ends_with(pattern) {
                    return Ok(true);
                }
                path = parent;
            }
        }
    }
    Ok(false)
}
