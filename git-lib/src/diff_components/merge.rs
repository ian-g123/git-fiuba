use std::collections::HashMap;

use crate::command_errors::CommandError;

pub fn merge_content(
    head_content: String,
    destin_content: String,
    common_content: String,
    head_name: &str,
    destin_name: &str,
) -> Result<(String, bool), CommandError> {
    let (common_not_changed_in_head, head_diffs) = get_diffs(&common_content, &head_content)?;
    let (common_not_changed_in_destin, destin_diffs) = get_diffs(&common_content, &destin_content)?;

    let line_count = common_content.split('\n').count();
    let (no_conflicts_content, conflicts_content) = merge_diffs(
        common_not_changed_in_head,
        head_diffs,
        common_not_changed_in_destin,
        destin_diffs,
        line_count,
    )?;

    let (merged_content, merge_conflicts) = build_output(
        no_conflicts_content,
        conflicts_content,
        head_name,
        destin_name,
    )?;
    Ok((merged_content, merge_conflicts))
}

fn build_output(
    no_conflicts_content: HashMap<usize, String>,
    conflicts_content: HashMap<usize, (Vec<String>, Vec<String>)>,
    head_name: &str,
    destin_name: &str,
) -> Result<(String, bool), CommandError> {
    let max_iter = match no_conflicts_content.keys().max() {
        Some(max) => max + 1,
        None => 1,
    };
    let mut merged_content = String::new();
    let merge_conflicts = !conflicts_content.is_empty();
    for i in 0..max_iter {
        match conflicts_content.get(&i) {
            Some((head_lines, destin_lines)) => {
                merged_content.push_str(&format!("<<<<<<< {}\n", head_name.to_string()));
                merged_content.push_str(&head_lines.join("\n"));
                merged_content.push_str(&format!("\n=======\n"));
                merged_content.push_str(&destin_lines.join("\n"));
                merged_content.push_str(&format!("\n>>>>>>> {}\n", destin_name.to_string()));
            }
            _ => {}
        };
        match no_conflicts_content.get(&i) {
            Some(line) => merged_content.push_str(&(line.to_owned() + "\n")),
            _ => {}
        };
    }
    merged_content.pop();
    Ok((merged_content, merge_conflicts))
}

fn merge_diffs(
    common_not_changed_in_head: HashMap<usize, String>,
    head_diffs: HashMap<usize, (Vec<String>, Vec<String>)>,
    common_not_changed_in_destin: HashMap<usize, String>,
    destin_diffs: HashMap<usize, (Vec<String>, Vec<String>)>,
    common_len: usize,
) -> Result<
    (
        HashMap<usize, String>,
        HashMap<usize, (Vec<String>, Vec<String>)>, //index line in common, (head line, destin line)
    ),
    CommandError,
> {
    // find intersection of common_not_changed_in_head and common_not_changed_in_destin
    let mut no_conflicts_content = HashMap::<usize, String>::new();
    let mut conflicts_content = HashMap::<usize, (Vec<String>, Vec<String>)>::new();
    let mut line_index = 0;
    let mut merge_index = 0;
    loop {
        let head_diffs_line_op = head_diffs.get(&line_index);
        let destin_diffs_line_op = destin_diffs.get(&line_index);

        match (head_diffs_line_op, destin_diffs_line_op) {
            (Some(head_diffs_line), Some(destin_diffs_line)) => {
                if head_diffs_line == destin_diffs_line {
                    for line in &head_diffs_line.0 {
                        no_conflicts_content.insert(merge_index, line.to_owned());
                        merge_index += 1;
                    }
                } else if just_adds(head_diffs_line) && just_adds(destin_diffs_line) {
                    for line in &head_diffs_line.0 {
                        no_conflicts_content.insert(merge_index, line.to_owned());
                        merge_index += 1;
                    }
                    for line in &destin_diffs_line.0 {
                        no_conflicts_content.insert(merge_index, line.to_owned());
                        merge_index += 1;
                    }
                } else {
                    let mut head_diff_buf = Vec::<String>::new();
                    let mut destin_diff_buf = Vec::<String>::new();
                    loop {
                        let head_common_line_op = common_not_changed_in_head.get(&line_index);
                        let destin_common_line_op = common_not_changed_in_destin.get(&line_index);

                        let head_diff_line_op = head_diffs.get(&line_index);
                        let destin_diff_line_op = destin_diffs.get(&line_index);

                        if let Some(head_diffs_line) = head_diff_line_op {
                            for line in &head_diffs_line.0 {
                                head_diff_buf.push(line.to_owned());
                            }
                        };
                        if let Some(destin_diffs_line) = destin_diff_line_op {
                            for line in &destin_diffs_line.0 {
                                destin_diff_buf.push(line.to_owned());
                            }
                        };

                        match (head_common_line_op, destin_common_line_op) {
                            (Some(head_comon_line), Some(destin_comon_line)) => {
                                conflicts_content
                                    .insert(merge_index, (head_diff_buf, destin_diff_buf));
                                merge_index += 1;
                                break;
                            }
                            (Some(head_comon_line), None) => {
                                head_diff_buf.push(head_comon_line.to_string());
                            }
                            (None, Some(destin_comon_line)) => {
                                destin_diff_buf.push(destin_comon_line.to_string());
                            }
                            (None, None) => {
                                if (head_diff_line_op.is_none() && destin_diff_line_op.is_none()) {
                                    conflicts_content
                                        .insert(merge_index, (head_diff_buf, destin_diff_buf));
                                    merge_index += 1;
                                    break;
                                }
                            }
                        };
                        line_index += 1;
                    }
                }
            }
            (None, Some(destin_diffs_line)) => {
                for line in &destin_diffs_line.0 {
                    no_conflicts_content.insert(merge_index, line.to_owned());
                    merge_index += 1;
                }
            }
            (Some(head_diffs_line), None) => {
                for line in &head_diffs_line.0 {
                    no_conflicts_content.insert(merge_index, line.to_owned());
                    merge_index += 1;
                }
            }
            (None, None) => {}
        }
        if line_index >= common_len {
            break;
        }
        let head_common_line_op = common_not_changed_in_head.get(&line_index);
        let destin_common_line_op = common_not_changed_in_destin.get(&line_index);

        match (head_common_line_op, destin_common_line_op) {
            (Some(head_comon_line), Some(_destin_comon_line)) => {
                no_conflicts_content.insert(merge_index, head_comon_line.to_string());
                merge_index += 1;
            }
            (_) => {}
        }

        line_index += 1;
    }

    Ok((no_conflicts_content, conflicts_content))
}

fn just_adds(diff_line: &(Vec<String>, Vec<String>)) -> bool {
    diff_line.1.is_empty()
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
    let common_lines: Vec<&str> = common_content.split('\n').collect::<Vec<&str>>();
    let other_lines = other_content.split('\n').collect::<Vec<&str>>();
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

#[cfg(test)]
mod test {
    use super::*;

    fn assert_merge_case(
        common_content: &str,
        head_content: &str,
        destin_content: &str,
        expected_output: &str,
        expect_merge_conflicts: bool,
    ) {
        let (merged_content, merge_conflicts) = merge_content(
            head_content.to_string(),
            destin_content.to_string(),
            common_content.to_string(),
            "HEAD",
            "origin",
        )
        .unwrap();

        assert_eq!(merged_content, expected_output.to_string());
        assert_eq!(merge_conflicts, expect_merge_conflicts);
    }

    #[test]
    fn no_changes() {
        assert_merge_case(
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            false,
        );
    }

    #[test]
    fn none_conflicting_changes() {
        assert_merge_case(
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3\nlinea 4",
            "linea 0\nlinea 1\nlinea 2\nlinea 3",
            "linea 0\nlinea 1\nlinea 2\nlinea 3\nlinea 4",
            false,
        );
    }

    #[test]
    fn conflicting_change_in_middle_line() {
        assert_merge_case(
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 5\nlinea 3",
            "linea 1\nlinea 6\nlinea 3",
            "linea 1\n<<<<<<< HEAD\nlinea 5\n=======\nlinea 6\n>>>>>>> origin\nlinea 3",
            true,
        );
    }

    #[test]
    fn deleted_first_line_in_both_branches() {
        assert_merge_case(
            "linea 0\nlinea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            false,
        );
    }

    #[test]
    fn delete_lines_and_begining_and_end_not_conflicting() {
        assert_merge_case(
            "linea 0\nlinea 1\nlinea 2\nlinea 3\nlinea 4",
            "linea 1\nlinea 2\nlinea 3\nlinea 4",
            "linea 0\nlinea 1\nlinea 2\nlinea 3",
            "linea 1\nlinea 2\nlinea 3",
            false,
        );
    }

    #[test]
    fn delete_and_modify_line_conflict() {
        assert_merge_case(
            "linea 0\nlinea 1",
            "linea 1",
            "linea 0 mod\nlinea 1",
            "<<<<<<< HEAD\n\n=======\nlinea 0 mod\n>>>>>>> origin\nlinea 1",
            true,
        );
    }

    #[test]
    fn no_common_content() {
        assert_merge_case(
            "linea 1\nlinea 2\nlinea 3",
            "linea 4\nlinea 5",
            "linea 6\nlinea 7",
            "<<<<<<< HEAD\nlinea 4\nlinea 5\n=======\nlinea 6\nlinea 7\n>>>>>>> origin",
            true,
        );
    }

    #[test]
    fn conflict_with_empty_head_content() {
        assert_merge_case(
            "linea 1\nlinea 2\nlinea 3",
            "",
            "linea 1\nlinea 2\nlinea 3",
            "",
            false,
        );
    }

    #[test]
    fn no_change_with_empty_head_and_destin_content() {
        assert_merge_case("", "", "", "", false);
    }

    #[test]
    fn test_both_branches_add_lines_in_between_common_lines() {
        assert_merge_case(
            "linea 1\nlinea 2",
            "linea 1\nadded linea A\nlinea 2",
            "linea 1\nadded linea B\nlinea 2",
            "linea 1\nadded linea A\nadded linea B\nlinea 2",
            false,
        );
    }

    #[test]
    fn test_one_branch_deletes_common_line_and_other_modifies_it() {
        assert_merge_case(
            "linea 1\nlinea 2",
            "linea 1\nmodified linea\nlinea 2",
            "linea 2",
            "modified linea\nlinea 2",
            false,
        );
    }

    #[test]
    fn test_both_branches_delete_different_lines() {
        assert_merge_case(
            "linea 1\nlinea 2\nline 3",
            "linea 1\nline 3",
            "linea 1\nlinea 2",
            "linea 1",
            false,
        );
    }
}
