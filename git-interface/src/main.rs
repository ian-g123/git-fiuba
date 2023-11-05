extern crate gtk;
use std::{
    collections::HashMap,
    env,
    io::{self},
};

use git::commands::commit::Commit;
use git_lib::{
    command_errors::CommandError, git_repository::GitRepository,
    objects::commit_object::CommitObject,
};
// use git_lib::*;
use gtk::{prelude::*, DrawingArea, Label, ListBox, ListBoxRow};

// colores para el grafo en el futuro
const GRAPH_COLORS: [(f64, f64, f64); 10] = [
    (1.0, 0.0, 0.0), // Rojo
    (0.0, 1.0, 0.0), // Verde
    (0.0, 0.0, 1.0), // Azul
    (1.0, 1.0, 0.0), // Amarillo
    (1.0, 0.5, 0.0), // Naranja
    (0.5, 0.0, 1.0), // Morado
    (0.0, 1.0, 1.0), // Cian
    (1.0, 0.0, 1.0), // Magenta
    (0.0, 0.0, 0.0), // Negro
    (1.0, 1.0, 1.0), // Blanco
];

struct Interface {
    builder: gtk::Builder,
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    let glade_src = include_str!("../git interface.glade");

    let interface = Interface {
        builder: gtk::Builder::from_string(glade_src),
    };

    // let commits = git::commands::log::Log::run_for_graph().unwrap();

    let window: gtk::Window = interface.builder.object("window app").unwrap();

    let _stagin_changes_list: gtk::ListBox =
        interface.builder.object("lista_staging_changes").unwrap();

    // cargamos la interfaz gráfica
    let _drawing_area: gtk::DrawingArea = interface.builder.object("drawing_area").unwrap();
    let _description_list: gtk::ListBox = interface.builder.object("description_list").unwrap();
    let _date_list: gtk::ListBox = interface.builder.object("date_list").unwrap();
    let _author_list: gtk::ListBox = interface.builder.object("author_list").unwrap();
    let _commits_hashes_list: gtk::ListBox = interface.builder.object("commit_hash_list").unwrap();

    // cargamos los botones
    let repo_git_path = "./git-interface/log".to_string();
    let output = &mut io::stdout();
    act_buttons(interface.builder, repo_git_path);

    // set_graph(
    //     &drawing_area,
    //     description_list,
    //     date_list,
    //     author_list,
    //     commits_hashes_list,
    //     commits,
    // );

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.show_all();

    gtk::main();
}

fn act_buttons<'a>(builder: gtk::Builder, repo_git_path: String) -> Result<(), CommandError> {
    let pull_button = build_button(&builder, "pull_button".to_string());
    let push_button = build_button(&builder, "push_button".to_string());
    let merge_button = build_button(&builder, "merge_button".to_string());
    let checkout_button = build_button(&builder, "checkout_button".to_string());
    let fetch_button = build_button(&builder, "fetch_button".to_string());
    let branch_button = build_button(&builder, "branch_button".to_string());

    connect_button(&pull_button, repo_git_path.clone(), "pull");
    connect_button(&push_button, repo_git_path.clone(), "push");
    connect_button(&merge_button, repo_git_path.clone(), "merge");
    connect_button(&checkout_button, repo_git_path.clone(), "checkout");
    connect_button(&fetch_button, repo_git_path.clone(), "fetch");
    connect_button(&branch_button, repo_git_path.clone(), "branch");

    Ok(())
}

fn build_button(builder: &gtk::Builder, name: String) -> gtk::Button {
    builder
        .object(name.as_str())
        .expect("No se pudo obtener el botón")
}

fn connect_button(button: &gtk::Button, repo_git_path: String, action: &str) {
    let action = action.to_owned();
    button.connect_clicked(move |_| {
        let output = &mut io::stdout();
        let result = GitRepository::open(&repo_git_path, output);

        match result {
            Ok(mut repo) => {
                match action.as_str() {
                    "fetch" => {
                        if let Err(err) = repo.fetch() {
                            eprintln!("Error al realizar fetch: {}", err);
                        }
                        println!("se presionó fetch");
                    }
                    "pull" => {
                        if let Err(err) = repo.pull() {
                            eprintln!("Error al realizar pull: {}", err);
                        }
                        println!("se presionó pull");
                    }
                    "push" => {
                        if let Err(err) = repo.push() {
                            eprintln!("Error al realizar push: {}", err);
                        }
                        println!("se presionó push");
                    }
                    "merge" => {
                        if let Err(err) = repo.merge() {
                            eprintln!("Error al realizar pull: {}", err);
                        }
                        println!("se presionó merge");
                    }
                    "checkout" => {
                        // if let Err(err) = repo.checkout() {
                        //     eprintln!("Error al realizar pull: {}", err);
                        // }
                        println!("se presionó checkout");
                    }
                    "branch" => {
                        // if let Err(err) = repo.branch() {
                        //     eprintln!("Error al realizar pull: {}", err);
                        // }
                        println!("se presionó branch");
                    }
                    "commit" => {
                        commit_function(&repo);
                        println!("se presionó commit");
                    }
                    _ => {
                        eprintln!("Acción no reconocida: {}", action);
                    }
                }
            }
            Err(err) => {
                eprintln!("Error al abrir el repositorio: {}", err);
            }
        }
    });
}

fn commit_function(repo: &GitRepository) {
    let mut message = String::new();
    let commit_msj_entry = gtk::Entry::new();

    println!("Ingrese el mensaje del commit: ");
    io::stdin()
        .read_line(&mut message)
        .expect("Error al leer el mensaje del commit");
    repo.commit(message.as_str()).unwrap();
    repo.commit(message, files, dry_run, reuse_commit_info, quiet);
}

// fn set_graph(
//     drawing_area: &DrawingArea,
//     description_list: ListBox,
//     date_list: ListBox,
//     author_list: ListBox,
//     commits_hashes_list: ListBox,
//     commits: Vec<(CommitObject, Option<String>)>,
// ) {
//     let mut hash_sons: HashMap<String, Vec<(f64, f64)>> = HashMap::new(); // hash, Vec<(x,y)> de los hijos
//     let mut hash_branches: HashMap<String, usize> = HashMap::new();
//     let mut identado: usize = 1;
//     for commit_and_branches in commits {
//         let mut commit = &commit_and_branches.0;
//         let y = add_row_to_list(&commit.message, &description_list);
//         identado = make_graph(
//             &drawing_area,
//             &mut hash_branches,
//             &mut hash_sons,
//             &mut identado,
//             &commit_and_branches,
//             y,
//         );
//         let mut commit = commit_and_branches.0;
//         add_row_to_list(&commit.timestamp.to_string(), &date_list);
//         add_row_to_list(&commit.author.to_string(), &author_list);
//         add_row_to_list(&commit.get_hash_string().unwrap(), &commits_hashes_list);
//     }
// }

// fn make_graph(
//     drawing_area: &DrawingArea,
//     hash_branches: &mut HashMap<String, usize>,
//     hash_sons: &mut HashMap<String, Vec<(f64, f64)>>,
//     identado: &mut usize,
//     commit: &(CommitObject, Option<String>),
//     y: i32,
// ) -> usize {
//     let commit_branch = commit.1.as_ref().unwrap();
//     //let commit_obj = &commit.0;
//     if !hash_branches.contains_key(commit_branch) {
//         hash_branches.insert(commit_branch.clone(), *identado);
//         *identado += 1;
//     }

//     let i = hash_branches.get(commit_branch).unwrap();
//     let index_color = i % GRAPH_COLORS.len();
//     let (c1, c2, c3): (f64, f64, f64) = GRAPH_COLORS[index_color];
//     let x: f64 = *i as f64 * 3.0;
//     let y: f64 = y as f64 * 1.0;

//     // Conéctate al evento "draw" del DrawingArea para dibujar
//     draw_commit_point(drawing_area, c1, c2, c3, x, y);

//     let commit_hash = &commit.0.get_hash_string().unwrap();
//     draw_lines_to_sons(hash_sons, commit_hash, drawing_area, c1, c2, c3, x, y);

//     for parent in &commit.0.get_parents() {
//         let sons_parent = hash_sons.entry(parent.clone()).or_default();
//         sons_parent.push((x, y));
//     }

//     return *identado;
// }

// fn draw_lines_to_sons(
//     hash_sons: &mut HashMap<String, Vec<(f64, f64)>>,
//     commit_hash: &String,
//     drawing_area: &DrawingArea,
//     c1: f64,
//     c2: f64,
//     c3: f64,
//     x: f64,
//     y: f64,
// ) {
//     if hash_sons.contains_key(commit_hash) {
//         for sons in hash_sons.get(commit_hash).unwrap() {
//             let sons_clone// extern crate gtk;
//             // use std::collections::HashMap;

//             // use git::*;
//             // use git_lib::objects::{author, commit_object::CommitObject};
//             // // use git_lib::*;
//             // use gtk::{prelude::*, DrawingArea, Label, ListBox, ListBoxRow};

//             // const GRAPH_COLORS: [(f64, f64, f64); 10] = [
//             //     (1.0, 0.0, 0.0), // Rojo
//             //     (0.0, 1.0, 0.0), // Verde
//             //     (0.0, 0.0, 1.0), // Azul
//             //     (1.0, 1.0, 0.0), // Amarillo
//             //     (1.0, 0.5, 0.0), // Naranja
//             //     (0.5, 0.0, 1.0), // Morado
//             //     (0.0, 1.0, 1.0), // Cian
//             //     (1.0, 0.0, 1.0), // Magenta
//             //     (0.0, 0.0, 0.0), // Negro
//             //     (1.0, 1.0, 1.0), // Blanco
//             // ];

//             // fn main() {
//             //     if gtk::init().is_err() {
//             //         println!("Failed to initialize GTK.");
//             //         return;
//             //     }

//             //     let commits = git::commands::log::Log::run_for_graph().unwrap();

//             //     let glade_src = include_str!("../../git interface.glade");
//             //     let builder = gtk::Builder::from_string(glade_src);
//             //     let window: gtk::Window = builder.object("window app").unwrap();

//             //     set_buttons();

//             //     let stagin_changes_list: gtk::ListBox = builder.object("lista_staging_changes").unwrap();

//             //     let drawing_area: gtk::DrawingArea = builder.object("drawing_area").unwrap();
//             //     let description_list: gtk::ListBox = builder.object("description_list").unwrap();
//             //     let date_list: gtk::ListBox = builder.object("date_list").unwrap();
//             //     let author_list: gtk::ListBox = builder.object("author_list").unwrap();
//             //     let commits_hashes_list: gtk::ListBox = builder.object("commit_hash_list").unwrap();

//             //     set_graph(
//             //         &drawing_area,
//             //         description_list,
//             //         date_list,
//             //         author_list,
//             //         commits_hashes_list,
//             //         commits,
//             //     );

//             //     window.connect_delete_event(|_, _| {
//             //         gtk::main_quit();
//             //         Inhibit(false)
//             //     });

//             //     window.show_all();

//             //     gtk::main();
//             // }

//             // fn set_graph(
//             //     drawing_area: &DrawingArea,
//             //     description_list: ListBox,
//             //     date_list: ListBox,
//             //     author_list: ListBox,
//             //     commits_hashes_list: ListBox,
//             //     commits: Vec<(CommitObject, Option<String>)>,
//             // ) {
//             //     let mut hash_sons: HashMap<String, Vec<(f64, f64)>> = HashMap::new(); // hash, Vec<(x,y)> de los hijos
//             //     let mut hash_branches: HashMap<String, usize> = HashMap::new();
//             //     let mut identado: usize = 1;
//             //     for commit_and_branches in commits {
//             //         let mut commit = &commit_and_branches.0;
//             //         let y = add_row_to_list(&commit.message, &description_list);
//             //         identado = make_graph(
//             //             &drawing_area,
//             //             &mut hash_branches,
//             //             &mut hash_sons,
//             //             &mut identado,
//             //             &commit_and_branches,
//             //             y,
//             //         );
//             //         let mut commit = commit_and_branches.0;
//             //         add_row_to_list(&commit.timestamp.to_string(), &date_list);
//             //         add_row_to_list(&commit.author.to_string(), &author_list);
//             //         add_row_to_list(&commit.get_hash_string().unwrap(), &commits_hashes_list);
//             //     }
//             // }

//             // fn make_graph(
//             //     drawing_area: &DrawingArea,
//             //     hash_branches: &mut HashMap<String, usize>,
//             //     hash_sons: &mut HashMap<String, Vec<(f64, f64)>>,
//             //     identado: &mut usize,
//             //     commit: &(CommitObject, Option<String>),
//             //     y: i32,
//             // ) -> usize {
//             //     let commit_branch = commit.1.as_ref().unwrap();
//             //     //let commit_obj = &commit.0;
//             //     if !hash_branches.contains_key(commit_branch) {
//             //         hash_branches.insert(commit_branch.clone(), *identado);
//             //         *identado += 1;
//             //     }

//             //     let i = hash_branches.get(commit_branch).unwrap();
//             //     let index_color = i % GRAPH_COLORS.len();
//             //     let (c1, c2, c3): (f64, f64, f64) = GRAPH_COLORS[index_color];
//             //     let x: f64 = *i as f64 * 3.0;
//             //     let y: f64 = y as f64 * 1.0;

//             //     // Conéctate al evento "draw" del DrawingArea para dibujar
//             //     draw_commit_point(drawing_area, c1, c2, c3, x, y);

//             //     let commit_hash = &commit.0.get_hash_string().unwrap();
//             //     draw_lines_to_sons(hash_sons, commit_hash, drawing_area, c1, c2, c3, x, y);

//             //     for parent in &commit.0.get_parents() {
//             //         let sons_parent = hash_sons.entry(parent.clone()).or_default();
//             //         sons_parent.push((x, y));
//             //     }

//             //     return *identado;
//             // }

//             // fn draw_lines_to_sons(
//             //     hash_sons: &mut HashMap<String, Vec<(f64, f64)>>,
//             //     commit_hash: &String,
//             //     drawing_area: &DrawingArea,
//             //     c1: f64,
//             //     c2: f64,
//             //     c3: f64,
//             //     x: f64,
//             //     y: f64,
//             // ) {
//             //     if hash_sons.contains_key(commit_hash) {
//             //         for sons in hash_sons.get(commit_hash).unwrap() {
//             //             let sons_clone = sons.clone();
//             //             drawing_area.connect_draw(move |_, context| {
//             //                 // Dibuja una línea en el DrawingArea
//             //                 context.set_source_rgb(c1, c2, c3);
//             //                 context.set_line_width(5.0);
//             //                 context.move_to(x, y);
//             //                 context.line_to(x, sons_clone.1.clone());
//             //                 context.stroke();
//             //                 Inhibit(false)
//             //             });
//             //             drawing_area.connect_draw(move |_, context| {
//             //                 // Dibuja una línea en el DrawingArea
//             //                 context.set_source_rgb(c1, c2, c3);
//             //                 context.set_line_width(5.0);
//             //                 context.move_to(x, sons_clone.1.clone());
//             //                 context.line_to(sons_clone.0.clone(), sons_clone.1.clone());
//             //                 context.stroke();
//             //                 Inhibit(false)
//             //             });
//             //         }
//             //     }
//             // }

//             // fn draw_commit_point(drawing_area: &DrawingArea, c1: f64, c2: f64, c3: f64, x: f64, y: f64) {
//             //     drawing_area.connect_draw(move |_, context| {
//             //         // Dibuja un punto en la posición (100, 100)
//             //         context.set_source_rgb(c1, c2, c3); // Establece el color en rojo
//             //         context.arc(x, y, 5.0, 0.0, 2.0 * std::f64::consts::PI); // Dibuja un círculo (punto)
//             //         context.fill();
//             //         Inhibit(false)
//             //     });
//             // }

//             // fn add_row_to_list(row_information: &String, row_list: &ListBox) -> i32 {
//             //     let label = Label::new(Some(&row_information));
//             //     let row_date = ListBoxRow::new();
//             //     row_date.add(&label);
//             //     row_list.add(&row_date);
//             //     row_date.allocation().y()
//             // }

//             // // fn add_

//             // // for _ in 1..50 {
//             // //     let drawing_area = DrawingArea::new();
//             // //     drawing_area.set_size_request(300, 300);
//             // //     drawing_area.connect_draw(|_, context| {
//             // //         // Dibuja una línea en el DrawingArea
//             // //         context.set_source_rgb(1.0, 1.0, 0.0);
//             // //         context.set_line_width(5.0);
//             // //         context.move_to(10.0, 10.0);
//             // //         context.line_to(190.0, 190.0);
//             // //         context.stroke();
//             // //         Inhibit(false)
//             // //     });
//             // //     stagin_changes_list.add(&drawing_area);
//             // // }
//             // // }

//             // fn set_buttons() {
//             //     // let commit: gtk::Button = builder.object("commit").unwrap();
//             //     // let more_options: gtk::Button = builder.object("more options").unwrap();
//             //     // let git_graph: gtk::Button = builder.object("git graph").unwrap();
//             //     // let refresh: gtk::Button = builder.object("refresh").unwrap();
//             //     // let mensaje_commit: gtk::Entry = builder.object("mensaje commit").unwrap();
//             // }

//             // // commit.connect_clicked(move |_| {
//             // //     if mensaje_commit.text().len() == 0 {
//             // //         let dialog = gtk::MessageDialog::new(
//             // //             Some(&window),
//             // //             gtk::DialogFlags::MODAL,
//             // //             gtk::MessageType::Error,
//             // //             gtk::ButtonsType::Ok,
//             // //             "No se ha ingresado un mensaje de commit",
//             // //         );
//             // //         dialog.run();
//             // //         dialog.hide();
//             // //     } else {
//             // //         let dialog = gtk::MessageDialog::new(
//             // //             Some(&window),
//             // //             gtk::DialogFlags::MODAL,
//             // //             gtk::MessageType::Info,
//             // //             gtk::ButtonsType::Ok,
//             // //             "Commit realizado con exito",
//             // //         );
//             // //         dialog.run();
//             // //         dialog.hide();
//             // //     }
//             // // });
//              = sons.clone();
//             drawing_area.connect_draw(move |_, context| {
//                 // Dibuja una línea en el DrawingArea
//                 context.set_source_rgb(c1, c2, c3);
//                 context.set_line_width(5.0);
//                 context.move_to(x, y);
//                 context.line_to(x, sons_clone.1.clone());
//                 context.stroke();
//                 Inhibit(false)
//             });
//             drawing_area.connect_draw(move |_, context| {
//                 // Dibuja una línea en el DrawingArea
//                 context.set_source_rgb(c1, c2, c3);
//                 context.set_line_width(5.0);
//                 context.move_to(x, sons_clone.1.clone());
//                 context.line_to(sons_clone.0.clone(), sons_clone.1.clone());
//                 context.stroke();
//                 Inhibit(false)
//             });
//         }
//     }
// }

// fn draw_commit_point(drawing_area: &DrawingArea, c1: f64, c2: f64, c3: f64, x: f64, y: f64) {
//     drawing_area.connect_draw(move |_, context| {
//         // Dibuja un punto en la posición (100, 100)
//         context.set_source_rgb(c1, c2, c3); // Establece el color en rojo
//         context.arc(x, y, 5.0, 0.0, 2.0 * std::f64::consts::PI); // Dibuja un círculo (punto)
//         context.fill();
//         Inhibit(false)
//     });
// }

// fn add_row_to_list(row_information: &String, row_list: &ListBox) -> i32 {
//     let label = Label::new(Some(&row_information));
//     let row_date = ListBoxRow::new();
//     row_date.add(&label);
//     row_list.add(&row_date);
//     row_date.allocation().y()
// }
