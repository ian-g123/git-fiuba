use std::{collections::HashMap, process::Command};

use crate::{changes_controller_components::merge_conflicts, command_errors::CommandError};

pub fn merge_content(
    head_content: String,
    destin_content: String,
    common_content: String,
) -> Result<(String, bool), CommandError> {
    let (common_not_changed_in_head, head_diffs) = get_diffs(&common_content, &head_content)?;
    let (common_not_changed_in_destin, destin_diffs) = get_diffs(&common_content, &destin_content)?;

    let (merged_content, merge_conflicts) = merge_difs(
        common_not_changed_in_head,
        head_diffs,
        common_not_changed_in_destin,
        destin_diffs,
    )?;
    Ok((merged_content, merge_conflicts))
}

fn merge_difs(
    common_not_changed_in_head: HashMap<usize, String>,
    head_diffs: HashMap<usize, (Vec<String>, Vec<String>)>,
    common_not_changed_in_destin: HashMap<usize, String>,
    destin_diffs: HashMap<usize, (Vec<String>, Vec<String>)>,
) -> Result<(String, bool), CommandError> {
    todo!()
}

/// Devuelve una tupla de dos HashMaps. El primero contiene las líneas que no cambiaron en "otro"
/// y el segundo contiene las diferencias entre "otro" y "común".\
/// Las diferencias están representadas por una tupla de dos vectores de strings. El primer vector
/// contiene las líneas nuevas lineas de "otro" y el segundo vector contiene las líneas de "común"
/// que cambiaron en "otro".
fn get_diffs(
    common_content: &String,
    other_content: &String,
) -> Result<
    (
        HashMap<usize, String>,
        HashMap<usize, (Vec<String>, Vec<String>)>,
    ),
    CommandError,
> {
    let mut common_not_changed_in_other = HashMap::<usize, String>::new();
    let mut other_diffs = HashMap::<usize, (Vec<String>, Vec<String>)>::new(); // index, (new_lines, discarted_lines)
    let common_lines: Vec<&str> = common_content.lines().collect::<Vec<&str>>();
    let other_lines = other_content.lines().collect::<Vec<&str>>();
    let mut common_index = 0;
    let mut other_index = 0;
    let mut common_buf = Vec::<String>::new();
    let mut other_buf = Vec::<String>::new();

    loop {
        let mut common_line_op = get_element(&common_lines, common_index);
        let mut other_line_op = get_element(&other_lines, other_index);
        if common_line_op.is_none() && other_line_op.is_none() {
            break;
        }
        if common_line_op == other_line_op {
            common_not_changed_in_other.insert(
                common_index,
                common_line_op
                    .ok_or(CommandError::MergeConflict("Error imposible".to_string()))?
                    .to_string(),
            );
            common_index += 1;
            other_index += 1;
        } else {
            let first_diff_other_index = other_index;
            let first_diff_common_index = common_index;
            loop {
                if let Some(common_line) = &common_line_op {
                    if let Some(other_line_index) = other_buf
                        .iter()
                        .position(|other_line| other_line == common_line)
                    {
                        let new_lines = other_buf[..other_line_index].to_vec();
                        let discarted_lines = common_buf.clone();
                        other_index = first_diff_other_index + new_lines.len();
                        other_diffs.insert(first_diff_common_index, (new_lines, discarted_lines));
                        break;
                    }
                    common_buf.push(common_line.to_string());
                }
                if let Some(other_line) = &other_line_op {
                    if let Some(common_line_index) = common_buf
                        .iter()
                        .position(|common_line| common_line == other_line)
                    {
                        let new_lines = other_buf.clone();
                        let discarted_lines = common_buf[..common_line_index].to_vec();
                        common_index = first_diff_common_index + discarted_lines.len();
                        other_diffs.insert(first_diff_common_index, (new_lines, discarted_lines));
                        break;
                    }
                    other_buf.push(other_line.to_string());
                }
                if common_line_op.is_none() && other_line_op.is_none() {
                    let new_lines = other_buf.clone();
                    let discarted_lines = common_buf.clone();
                    common_index = first_diff_common_index + discarted_lines.len();
                    other_index = first_diff_other_index + new_lines.len();
                    other_diffs.insert(first_diff_common_index, (new_lines, discarted_lines));
                    break;
                }

                if common_index < common_lines.len() {
                    common_index += 1;
                }
                if other_index < other_lines.len() {
                    other_index += 1;
                }
                common_line_op = get_element(&common_lines, common_index);
                other_line_op = get_element(&other_lines, other_index);
            }
            common_buf.clear();
            other_buf.clear();
        }
    }

    Ok((common_not_changed_in_other, other_diffs))
}

fn get_element(vector: &Vec<&str>, index: usize) -> Option<String> {
    if index >= vector.len() {
        None
    } else {
        Some(vector[index].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_differences() {
        let common_content = "linea 1\nlinea 2".to_string();
        let other_content = "linea 1\nlinea 2".to_string();
        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();
        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 0);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(common_not_changed_in_other.get(&1).unwrap(), "linea 2");
    }

    #[test]
    fn difference_in_middle_line() {
        let common_content = "linea 1\nlinea 2\nlinea 3".to_string();
        let other_content = "linea 1\nlinea 2 mod\nlinea 3".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(common_not_changed_in_other.get(&2).unwrap(), "linea 3");
        assert_eq!(other_diffs.get(&1).unwrap().0[0], "linea 2 mod".to_string());
        assert_eq!(other_diffs.get(&1).unwrap().1[0], "linea 2".to_string());
    }

    #[test]
    fn difference_in_last_line() {
        let common_content = "linea 1\nlinea 2".to_string();
        let other_content = "linea 1\nlinea 2 mod".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 1);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(other_diffs.get(&1).unwrap().0[0], "linea 2 mod".to_string());
        assert_eq!(other_diffs.get(&1).unwrap().1[0], "linea 2".to_string());
    }

    #[test]
    fn common_ends_before_with_matching_line() {
        let common_content = "linea 1".to_string();
        let other_content = "linea 1\nlinea 2 extra".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 1);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(
            other_diffs.get(&1).unwrap().0[0],
            "linea 2 extra".to_string()
        );
        assert!(other_diffs.get(&1).unwrap().1.is_empty());
    }

    #[test]
    fn other_ends_before_with_matching_line() {
        let common_content = "linea 1\nlinea 2 extra".to_string();
        let other_content = "linea 1".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 1);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(
            other_diffs.get(&1).unwrap().1[0],
            "linea 2 extra".to_string()
        );
        assert!(other_diffs.get(&1).unwrap().0.is_empty());
    }

    #[test]
    fn other_ends_before_with_not_matching_line() {
        let common_content = "linea 1\nlinea 2 common\nlinea 3 extra".to_string();
        let other_content = "linea 1\nlinea 2 other".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 1);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(
            other_diffs.get(&1).unwrap().0[0],
            "linea 2 other".to_string()
        );
        assert_eq!(
            other_diffs.get(&1).unwrap().1[0],
            "linea 2 common".to_string()
        );
        assert_eq!(
            other_diffs.get(&1).unwrap().1[1],
            "linea 3 extra".to_string()
        );
    }

    #[test]
    fn middle_line_missing_in_other() {
        let common_content = "linea 1\nlinea 2\nlinea 3".to_string();
        let other_content = "linea 1\nlinea 3".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(common_not_changed_in_other.get(&2).unwrap(), "linea 3");
        assert!(other_diffs.get(&1).unwrap().0.is_empty());
        assert_eq!(other_diffs.get(&1).unwrap().1[0], "linea 2".to_string());
    }

    #[test]
    fn middle_line_missing_in_common() {
        let common_content = "linea 1\nlinea 3".to_string();
        let other_content = "linea 1\nlinea 2\nlinea 3".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(common_not_changed_in_other.get(&1).unwrap(), "linea 3");
        assert!(other_diffs.get(&1).unwrap().1.is_empty());
        assert_eq!(other_diffs.get(&1).unwrap().0[0], "linea 2".to_string());
    }

    #[test]
    fn initial_line_missing_in_other() {
        let common_content = "linea 1\nlinea 2\nlinea 3".to_string();
        let other_content = "linea 2\nlinea 3".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&1).unwrap(), "linea 2");
        assert_eq!(common_not_changed_in_other.get(&2).unwrap(), "linea 3");
        assert_eq!(other_diffs.get(&0).unwrap().1[0], "linea 1".to_string());
        assert!(other_diffs.get(&0).unwrap().0.is_empty());
    }

    #[test]
    fn initial_line_missing_in_common() {
        let common_content = "linea 2\nlinea 3".to_string();
        let other_content = "linea 1\nlinea 2\nlinea 3".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 2");
        assert_eq!(common_not_changed_in_other.get(&1).unwrap(), "linea 3");
        assert_eq!(other_diffs.get(&0).unwrap().0[0], "linea 1".to_string());
        assert!(other_diffs.get(&0).unwrap().1.is_empty());
    }
}
