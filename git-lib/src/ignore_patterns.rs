use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    command_errors::CommandError,
    git_repository::{get_path_str, next_line},
    join_paths,
    logger::{self, Logger},
    utils::aux::get_name,
};

const BACKSLASH: char = '\\';
const SLASH: char = '/';
const ASTERISK: char = '*';
const NEGATE: char = '!';

#[derive(Clone)]
pub struct GitignorePatterns {
    patterns: Vec<(String, Vec<(usize, Pattern)>)>,
}

impl GitignorePatterns {
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

        let mut patterns_matched: Vec<(String, usize, Pattern)> = Vec::new();
        for (dir_level, pattern_vec) in self.patterns.iter() {
            logger.log(&format!("Gitignore path: {}", dir_level));
            for (line_number, pattern) in pattern_vec {
                logger.log(&format!(
                    "Line: {}, pattern: {}",
                    line_number,
                    pattern.get_pattern_read()
                ));

                let base_path = if dir_level.ends_with(".gitignore") {
                    dir_level[..dir_level.len() - 10].to_string()
                } else {
                    dir_level[..dir_level.len() - 17].to_string()
                };

                if matches_pattern(&path, pattern, &base_path, logger)? {
                    /* patterns_matched.push((
                        dir_level.to_string(),
                        line_number.to_owned(),
                        pattern.to_owned(),
                    ));*/
                    logger.log("matches pattern");
                    if pattern.negate_pattern() {
                        logger.log(&format!("negate pattern"));

                        negate_pattern = Some((
                            dir_level.to_string(),
                            line_number.to_owned(),
                            pattern.clone(),
                        ));
                    }
                    /*else if negate_pattern.is_some() {
                        negate_pattern = None;
                    } */
                    else {
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
        /*Ok(patterns_matched.pop()) */
        Ok(last_pattern)
    }
}

fn get_gitignore_files(
    git_path: &str,
    working_dir_path: &str,
    logger: &mut Logger,
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
            logger.log("Existe exclude");

            gitignore_files.insert(0, exclude_path);
        }
    }

    Ok(gitignore_files)
}

fn add_gitignore_patterns(
    gitignore_path: &str,
    patterns: &mut Vec<(String, Vec<(usize, Pattern)>)>,
    logger: &mut Logger,
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
            logger.log(&format!("i = {}, char = {}", i, character));
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
                        if !(i == last_index - 1 && line[last_index..].starts_with(ASTERISK)) {
                            is_relative = true;
                        }
                    }
                    logger.log(&format!("i = {}", i));
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

        patterns_hashmap.push((line_number + 1, pattern));
        line_number += 1;
    }
    _ = patterns.push((gitignore_path.to_string(), patterns_hashmap));
    Ok(())
}

#[derive(Clone, Debug)]
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

fn matches_pattern(
    path: &str,
    pattern: &Pattern,
    base_path: &str,
    logger: &mut Logger,
) -> Result<bool, CommandError> {
    /* let path = {
        if pattern.is_relative() {
            logger.log(&format!("es relativo"));

            path.to_string()
        } else {
            join_paths!(base_path, path).ok_or(CommandError::FileCreationError(format!(
                " No se pudo formar la ruta del archivo"
            )))?
        }
    }; */
    match pattern {
        Pattern::StartsWith(_, pattern, is_relative, _) => {
            logger.log(&format!(
                "STARTS WITH --> path: {}, pattern: {}, is_relative: {}, is_dir: {}",
                path,
                pattern,
                is_relative,
                is_dir(pattern)
            ));

            if is_relative.to_owned() || is_dir(pattern) {
                if path.starts_with(pattern) {
                    logger.log(&format!("is_rel or dir, matches",));
                    return Ok(true);
                }
                return Ok(false);
            }

            let mut e = pattern.len();
            //let mut s = 0;
            let mut index = 0;

            if !is_relative.to_owned() || !is_dir(pattern) {
                for _ in path.chars() {
                    if path[index..].starts_with(pattern) {
                        logger.log(&format!("e+index = {}", e + index));
                        if (index > 0 && !path[index - 1..].starts_with("/"))
                        /* || (!pattern.ends_with("/")
                            && e + index < path.len() - 1
                            && &path[e + index..] == "/")
                        || !(e + index < path.len() - 1) */
                        {
                            return Ok(false);

                            //s = index; //if index > 0 { index - 1 } else { 0 };
                            //break;
                        }
                        return Ok(true);
                    }
                    index += 1;
                }
            }

            /* logger.log(&format!("e={}", e));
            if let Some(rest) = path.get(e..) {
                if is_dir(pattern) && !rest.starts_with("/") {
                    return Ok(false);
                }
            } */
            return Ok(false);
        }
        Pattern::EndsWith(_, pattern, is_relative, _) => {
            logger.log(&format!(
                "ENDS WITH --> path: {}, pattern: {}, is_dir: {}, is_relative: {}",
                path,
                pattern,
                is_dir(pattern),
                is_relative.to_owned()
            ));
            /* if !is_dir(pattern) && path.ends_with(pattern) {
                return Ok(true);
            } else if is_dir(pattern) { */
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
                //logger.log(&format!("Es dir. s={}", s));
                let e = s + pattern.len();
                /* if s > 0 {
                    e -= 1;
                } */

                logger.log(&format!("s={},e={}", s, e));
                logger.log(&format!(
                    "Conditions: (1) {} (2) {} (3) {}",
                    e < path.len() - 1 && !path[e..].starts_with("/"),
                    is_relative.to_owned() && s > 0 && path[..s].contains("/"),
                    !is_relative.to_owned()
                        && is_dir(pattern)
                        && s > 0
                        && !path[..s].ends_with("/")
                ));

                if (e < path.len() - 1 && !path[e..].starts_with("/") && !is_dir(pattern))
                    || (is_relative.to_owned() && s > 0 && path[..s].contains("/"))
                /* || (!is_relative.to_owned()
                && is_dir(pattern)
                && s > 0
                && !path[..s].ends_with("/")) */
                {
                    return Ok(false);
                }
                return Ok(true);
            }
        }
        Pattern::MatchesAsterisk(_, start, end, _, _) => {
            logger.log(&format!(
                "MATCHES --> path: {}, starts: {}, ends:{}",
                path, start, end
            ));

            /* if path.len() > start.len() + end.len() {
                if path[start.len()..end.len()].contains("/") {
                    return Ok(false);
                }
            } */
            if path.starts_with(start) && path.ends_with(end) {
                return Ok(true);
            }
        }
        Pattern::RelativeToDirLevel(_, pattern, _) => {
            logger.log(&format!(
                "RELATIVE --> path: {}, pattern: {}",
                path, pattern
            ));
            if path.starts_with(pattern) {
                logger.log(&format!("comienza con el patrón",));
                if let Some(rest) = path.get(pattern.len()..) {
                    logger.log(&format!("rest: {}", rest));
                    if rest != "" && !rest.starts_with("/") && !is_dir(pattern) {
                        logger.log(&format!("no coincide"));

                        return Ok(false);
                    }
                }
                return Ok(true);
            }
        }
        Pattern::NotRelativeToDirLevel(_, pattern, _) => {
            logger.log(&format!(
                "NOT RELATIVE--> path: {}, pattern: {}",
                path, pattern
            ));

            let mut index = 0;

            if path.contains(pattern) {
                for _ in path.chars() {
                    if path[index..].starts_with(pattern) {
                        if let Some(rest) = path.get(index + pattern.len()..) {
                            logger.log(&format!("rest: {}", rest));

                            if rest != "" && !rest.starts_with("/") && !is_dir(pattern) {
                                logger.log("No cumple final");
                                return Ok(false);
                            }
                        }
                        if let Some(rest) = path.get(..index) {
                            logger.log(&format!("rest: {}", rest));

                            if rest != "" && !rest.ends_with("/") {
                                logger.log("No cumple principio");

                                return Ok(false);
                            }
                        }
                        return Ok(true);
                    }
                    index += 1;
                }
            }
        }
    }
    Ok(false)
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
            logger.log(&format!("Buscando .gitignore antes: {}", entry_name));

            if let Some(path) = entry_name.strip_prefix(base_path) {
                logger.log(&format!("Buscando .gitignore después: {}", path));
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

fn is_dir(path: &str) -> bool {
    let obj_path = Path::new(path);
    path.ends_with("/")
}
