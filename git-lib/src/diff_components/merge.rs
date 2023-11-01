use std::collections::HashMap;

use crate::command_errors::CommandError;

pub fn merge_content(
    head_content: String,
    destin_content: String,
    common_content: String,
) -> Result<(String, bool), CommandError> {
    let (common_not_changed_in_head, head_diffs) = get_diffs(&common_content, &head_content)?;

    // let mut merged_lines = Vec::<String>::new();
    // let head_lines = head_content.lines().collect::<Vec<&str>>();
    // let destin_lines = destin_content.lines().collect::<Vec<&str>>();
    // let common_lines = common_content.lines().collect::<Vec<&str>>();

    // let mut head_index = 0;
    // let mut destin_index = 0;
    // let mut head_buf = Vec::<&str>::new();
    // let mut destin_buf = Vec::<&str>::new();
    // while (head_index < head_lines.len()) && (destin_index < destin_lines.len()) {
    //     let head_line = head_lines[head_index];
    //     let destin_line = destin_lines[destin_index];
    //     if head_line == destin_line {
    //         merged_lines.push(head_line.to_string());
    //         head_index += 1;
    //         destin_index += 1;
    //     } else {
    //         if let Some(matching_head_line_index) = head_buf
    //             .clone()
    //             .into_iter()
    //             .position(|head_line| head_line == destin_line)
    //         {
    //             let mut head_changes = head_buf[0..matching_head_line_index].to_vec();
    //             let mut destin_changes = destin_buf[0..destin_index].to_vec();
    //             merged_lines.push("<<<<<<< HEAD".to_string());
    //             merged_lines.append(&mut head_changes.iter().map(|s| s.to_string()).collect());
    //             merged_lines.push("=======".to_string());
    //             merged_lines.append(&mut destin_changes.iter().map(|s| s.to_string()).collect());
    //             merged_lines.push(">>>>>>>".to_string());
    //         }
    //         head_buf.push(head_line);
    //         destin_buf.push(destin_line);
    //     }
    // }
    todo!();
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

    while (common_index < common_lines.len()) && (other_index < other_lines.len()) {
        let mut common_line = common_lines[common_index];
        let mut other_line = other_lines[other_index];

        if common_line == other_line {
            common_not_changed_in_other.insert(common_index, common_line.to_string());
        } else {
            loop {
                if let Some(other_line_index) = other_buf
                    .iter()
                    .position(|other_line| other_line == common_line)
                {
                    let discarted_lines = other_buf[..other_line_index].to_vec();
                    // other_index = other_index - other_buf.len() + other_line_index;
                    other_index = other_index - other_buf.len() + other_line_index;
                    common_index -= 1;
                    other_diffs.insert(common_index, (discarted_lines.clone(), common_buf.clone()));
                    break;
                } else if let Some(common_line_index) = common_buf
                    .iter()
                    .position(|other_line| other_line == common_line)
                {
                    let discarted_lines = common_buf[..common_line_index].to_vec();
                    common_index -= common_buf.len() - common_line_index;
                    // common_index = common_index - common_buf.len() + common_line_index;
                    other_index -= 1;
                    other_diffs.insert(common_index, (discarted_lines.clone(), common_buf.clone()));
                    break;
                }

                common_buf.push(common_line.to_string());
                other_buf.push(other_line.to_string());

                if !(common_index < common_lines.len() || other_index < other_lines.len()) {
                    break;
                }

                if common_index < common_lines.len() - 1 {
                    common_index += 1;
                    common_line = common_lines[common_index];
                }
                if other_index < other_lines.len() - 1 {
                    other_index += 1;
                    other_line = other_lines[other_index];
                }
            }
        }
        common_index += 1;
        other_index += 1;
    }

    Ok((common_not_changed_in_other, other_diffs))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_diffs_test() {
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
    fn get_diffs_test_2() {
        let common_content = "linea 1\nlinea 2\nlinea 3".to_string();
        let other_content = "linea 1\nlinea 2 mod\nlinea 3".to_string();

        let (common_not_changed_in_other, other_diffs) =
            get_diffs(&common_content, &other_content).unwrap();

        assert_eq!(common_not_changed_in_other.len(), 2);
        assert_eq!(other_diffs.len(), 1);
        assert_eq!(common_not_changed_in_other.get(&0).unwrap(), "linea 1");
        assert_eq!(common_not_changed_in_other.get(&2).unwrap(), "linea 3");
        assert_eq!(other_diffs.get(&1).unwrap().0[0], "linea 2 mod".to_string());
        //assert_eq!(other_diffs.get(&1).unwrap().1[0], "linea 2".to_string());
    }

    #[test]
    fn get_diffs_test_3() {
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
}

// fn common_line_differs_other_line(common_lines, other_lines, common_index, other_index){

// }

// fn flush_buffers(
//     common_buf: &mut Vec<String>,
//     matching_common_line_index: usize,
//     other_buf: &mut Vec<String>,
//     other_diffs: &mut HashMap<usize, (Vec<String>, Vec<String>)>,
//     common_index: &mut usize,
//     common_not_changed_in_other: &mut HashMap<usize, String>,
//     other_line: &str,
// ) {
//     let discarted_lines = common_buf[..matching_common_line_index].to_vec();
//     let new_lines = other_buf.to_vec();
//     other_diffs.insert(
//         *common_index - common_buf.len(),
//         (new_lines.clone(), discarted_lines.clone()),
//     );
//     common_buf.clear();
//     other_buf.clear();
//     *common_index -= common_buf.len() - matching_common_line_index;
//     common_not_changed_in_other.insert(*common_index, other_line.to_string());
// }

// fn get_diffs(
//     common_content: &String,
//     other_content: &String,
// ) -> Result<
//     (
//         HashMap<usize, String>,
//         HashMap<usize, (Vec<String>, Vec<String>)>,
//     ),
//     CommandError,
// > {
//     let mut common_not_changed_in_other = HashMap::<usize, String>::new();
//     let mut other_diffs = HashMap::<usize, (Vec<String>, Vec<String>)>::new();

//     let other_lines = other_content.lines().collect::<Vec<&str>>();
//     let common_lines: Vec<&str> = common_content.lines().collect::<Vec<&str>>();

//     let mut other_index = 0;
//     let mut common_index = 0;

//     let mut common_buf = Vec::<String>::new();
//     let mut other_buf = Vec::<String>::new();

//     while (common_index < common_lines.len()) && (other_index < other_lines.len()) {
//         let mut common_line = common_lines[common_index];
//         let mut other_line = other_lines[other_index];
//         if common_line == other_line {
//             common_not_changed_in_other.insert(common_index, common_line.to_string());
//         } else {
//             loop {
//                 if common_line == other_line {
//                     let discarted_lines = common_buf.to_vec();
//                     let new_lines = other_buf.to_vec();
//                     other_diffs.insert(
//                         common_index - common_buf.len(),
//                         (new_lines.clone(), discarted_lines.clone()),
//                     );
//                     common_buf.clear();
//                     other_buf.clear();
//                     common_index -= common_buf.len();
//                     common_not_changed_in_other.insert(common_index, common_line.to_string());
//                     break;
//                 }
//                 if let Some(matching_common_line_index) = common_buf
//                     .iter()
//                     .position(|common_line| common_line == other_line)
//                 {
//                     flush_buffers(
//                         &mut common_buf,
//                         matching_common_line_index,
//                         &mut other_buf,
//                         &mut other_diffs,
//                         &mut common_index,
//                         &mut common_not_changed_in_other,
//                         other_line,
//                     );
//                     break;
//                 } else if let Some(matching_other_line_index) = other_buf
//                     .iter()
//                     .position(|other_line| other_line == common_line)
//                 {
//                     flush_buffers(
//                         &mut other_buf,
//                         matching_other_line_index,
//                         &mut common_buf,
//                         &mut other_diffs,
//                         &mut other_index,
//                         &mut common_not_changed_in_other,
//                         common_line,
//                     );
//                     break;
//                 } else {
//                     common_buf.push(common_line.to_string());
//                     other_buf.push(other_line.to_string());
//                 }

//                 if !(common_index < common_lines.len() || other_index < other_lines.len()) {
//                     break;
//                 }
//                 common_index += 1;
//                 other_index += 1;
//                 if common_index < common_lines.len() {
//                     common_line = common_lines[common_index];
//                 }
//                 if other_index < other_lines.len() {
//                     other_line = other_lines[other_index];
//                 }
//             }
//         }
//         common_index += 1;
//         other_index += 1;
//     }

//     Ok((common_not_changed_in_other, other_diffs))
// }
