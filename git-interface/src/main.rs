extern crate gtk;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::{self, Write},
    rc::Rc,
};

use gtk::{
    prelude::*, Button, DrawingArea, Label, ListBox, ListBoxRow, Orientation, Window, WindowType,
};

use git::commands::push::Push;
use git_lib::{
    changes_controller_components::{
        changes_controller::ChangesController, long_format::sort_hashmap,
    },
    command_errors::CommandError,
    git_repository::GitRepository,
    objects::{commit_object::CommitObject, git_object::GitObjectTrait},
};

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
    repo_git_path: String,
    staging_changes: Rc<RefCell<HashSet<String>>>,
    unstaging_changes: Rc<RefCell<HashSet<String>>>,
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("../git interface.glade");
    let repo_git_path = "./git-interface/log".to_string();

    let mut output = io::stdout();
    let mut repo = match GitRepository::open(&repo_git_path, &mut output) {
        Ok(repo) => repo,
        Err(_) => {
            eprintln!("No se pudo conectar satisfactoriamente a un repositorio Git.");
            return;
        }
    };

    let (staged_files, changes_file) = staged_area_func(repo_git_path.clone()).unwrap();
    let staging_changes = Rc::new(RefCell::new(staged_files));
    let unstaging_changes = Rc::new(RefCell::new(changes_file));

    let mut interface = Interface {
        builder: gtk::Builder::from_string(glade_src),
        repo_git_path,
        staging_changes,
        unstaging_changes,
    };

    let commits = match repo.get_log(true) {
        Ok(commits) => commits,
        Err(_) => {
            eprintln!("No se pudo conectar satisfactoriamente a un repositorio Git.");
            return;
        }
    };

    let mut window: gtk::Window = interface.builder.object("window app").unwrap();

    let _stagin_changes_list: gtk::ListBox = interface.builder.object("staging_list").unwrap();

    // cargamos la interfaz gráfica
    let drawing_area: gtk::DrawingArea = interface.builder.object("drawing_area").unwrap();
    let description_list: gtk::ListBox = interface.builder.object("description_list").unwrap();
    let date_list: gtk::ListBox = interface.builder.object("date_list").unwrap();
    let author_list: gtk::ListBox = interface.builder.object("author_list").unwrap();
    let commits_hashes_list: gtk::ListBox = interface.builder.object("commit_hash_list").unwrap();

    // cargamos los botones
    interface.buttons_activation();

    interface.build_ui(&mut window);

    set_right_area(
        &drawing_area,
        description_list,
        date_list,
        author_list,
        commits_hashes_list,
        commits,
    );

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.show_all();

    gtk::main();
}

impl Interface {
    fn buttons_activation<'a>(&mut self) -> Result<(), CommandError> {
        let buttons = [
            ("pull", self.build_button("pull_button".to_string())),
            ("push", self.build_button("push_button".to_string())),
            ("checkout", self.build_button("checkout_button".to_string())),
            ("fetch", self.build_button("fetch_button".to_string())),
            ("branch", self.build_button("branch_button".to_string())),
            ("commit", self.build_button("commit_button".to_string())),
        ];

        for button in buttons.iter() {
            self.connect_button(button.0.to_string(), &button.1)?;
        }

        Ok(())
    }

    fn build_button(&self, name: String) -> gtk::Button {
        self.builder
            .object(name.as_str())
            .expect(format!("No se pudo obtener el botón {}", name).as_str())
    }

    fn connect_button(
        &self,
        button_action: String,
        button: &gtk::Button,
    ) -> Result<(), CommandError> {
        let repo_git_path = self.repo_git_path.clone();
        let commit_entry_msg: gtk::Entry = self
            .builder
            .object("entrada_de_mensaje")
            .expect("No se pudo obtener la entrada de mensaje");
        let message: gtk::glib::GString = commit_entry_msg.text();

        button.connect_clicked(move |_| {
            let commit_msg = message.to_string();
            let output = io::stdout();
            let mut binding = &output;
            let mut repo = match GitRepository::open(&repo_git_path, &mut binding) {
                Ok(repo) => repo,
                Err(_) => {
                    eprintln!("No se pudo conectar satisfactoriamente a un repositorio Git.");
                    return;
                }
            };

            match button_action.as_str() {
                "pull" => {
                    if let Err(err) = repo.pull() {
                        eprintln!("Error en al presionar el botón pull: {}", err);
                    }
                    println!("se presionó pull");
                }
                "push" => {
                    let mut binding_for_push = &output;
                    push_function(&mut binding_for_push);
                    println!("se presionó push");
                }
                "fetch" => {
                    if let Err(err) = repo.fetch() {
                        eprintln!("Error en al presionar el botón fetch: {}", err);
                    }
                    println!("se presionó fetch");
                }
                "branch" => {
                    // Aquí puedes agregar tu lógica para branch
                    println!("se presionó branch");
                }
                "commit" => {
                    println!("se presionó commit");
                    commit_function(&repo, commit_msg);
                }
                _ => {
                    eprintln!("Acción no reconocida: {}", button_action);
                }
            }
        });
        Ok(())
    }

    fn build_ui(self, window: &gtk::Window) {
        let staging_changes: gtk::ListBox = self.builder.object("staging_list").unwrap();
        let unstaging_changes: gtk::ListBox = self.builder.object("unstaging_list").unwrap();

        unstaging_changes.foreach(|child| {
            unstaging_changes.remove(child);
        });
        staging_changes.foreach(|child| {
            staging_changes.remove(child);
        });

        for file in self.unstaging_changes.borrow().iter() {
            let file = file.clone();
            let box_outer = gtk::Box::new(Orientation::Horizontal, 0);

            let label = Label::new(Some(&format!("{}", file)));
            let button_stage = Button::with_label("stage");

            box_outer.pack_start(&label, true, true, 0);
            box_outer.pack_end(&button_stage, false, false, 0);

            unstaging_changes.add(&box_outer);

            let unstaging_changes = Rc::clone(&self.unstaging_changes);
            let staging_changes = Rc::clone(&self.staging_changes);
            let builder = self.builder.clone();
            let window = window.clone();

            window.show_all();

            let repo_git_path = self.repo_git_path.clone();
            button_stage.connect_clicked(move |_| {
                _ = unstaging_changes.borrow_mut().take(&file);
                staging_changes.borrow_mut().insert(file.clone());

                let interface = Interface {
                    builder: builder.clone(),
                    repo_git_path: repo_git_path.to_string(),
                    staging_changes: Rc::clone(&staging_changes),
                    unstaging_changes: Rc::clone(&unstaging_changes),
                };
                interface.build_ui(&window);
            });
        }

        for file in self.staging_changes.borrow().iter() {
            let file = file.clone();
            let box_outer = gtk::Box::new(Orientation::Horizontal, 0);

            let label = Label::new(Some(&format!("{}", file)));
            let button_unstage = Button::with_label("unstage");

            box_outer.pack_start(&label, true, true, 0);
            box_outer.pack_end(&button_unstage, false, false, 0);

            staging_changes.add(&box_outer);

            let unstaging_changes = Rc::clone(&self.unstaging_changes);
            let staging_changes = Rc::clone(&self.staging_changes);
            let builder = self.builder.clone();
            let window = window.clone();

            window.show_all();

            let repo_git_path = self.repo_git_path.clone();
            button_unstage.connect_clicked(move |_| {
                _ = staging_changes.borrow_mut().take(&file);
                unstaging_changes.borrow_mut().insert(file.clone());

                let interface = Interface {
                    builder: builder.clone(),
                    repo_git_path: repo_git_path.to_string(),
                    staging_changes: Rc::clone(&staging_changes),
                    unstaging_changes: Rc::clone(&unstaging_changes),
                };
                interface.build_ui(&window);
            });
        }
    }
}

fn staged_area_func(
    repo_git_path: String,
) -> Result<(HashSet<String>, HashSet<String>), CommandError> {
    // staged_area, unstage_area
    let mut output = io::stdout();
    let mut repo = GitRepository::open(&repo_git_path, &mut output).unwrap();
    let db = repo.db().unwrap();

    let last_commit_tree = match repo.get_last_commit_tree() {
        Ok(tree) => tree,
        Err(err) => {
            eprintln!("Error al obtener el último commit: {}", err);
            return Err(CommandError::FileWriteError(err.to_string()));
        }
    };

    let changes_controller =
        ChangesController::new(&db, &repo_git_path, repo.get_logger(), last_commit_tree).unwrap();

    let changes_to_be_commited_vec = sort_hashmap(changes_controller.get_changes_to_be_commited());
    let changes_to_be_commited: HashSet<String> = changes_to_be_commited_vec
        .into_iter()
        .map(|(s, _)| s)
        .collect();

    let changes_not_staged_vec = sort_hashmap(changes_controller.get_changes_not_staged());
    let mut changes_not_staged: HashSet<String> =
        changes_not_staged_vec.into_iter().map(|(s, _)| s).collect();

    let untracked_files_vec = changes_controller.get_untracked_files();

    println!("untracked_files_vec: {:?}", untracked_files_vec);

    changes_not_staged.extend(untracked_files_vec.iter().cloned());

    return Ok((changes_to_be_commited, changes_not_staged));
}

fn commit_function(repo: &GitRepository, commit_msg: String) {
    if commit_msg.is_empty() {
        let window = Window::new(WindowType::Toplevel);
        window.set_title("Empty commit message");
        window.set_default_size(300, 200);

        let dialog = gtk::MessageDialog::new(
            Some(&window),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Info,
            gtk::ButtonsType::Close,
            "Ingrese un mensaje, por favor",
        );

        dialog.connect_response(|dialog, _| {
            dialog.hide();
        });

        eprintln!("No se ha ingresado un mensaje de commit");
        dialog.run();
        return;
    }
}

fn push_function(output: &mut dyn Write) {
    let push = Push::new_default(output).unwrap();
    push.run(output).unwrap();
}

fn set_right_area(
    _drawing_area: &DrawingArea, // TODO: implementar el grafo para la entrega final
    description_list: ListBox,
    date_list: ListBox,
    author_list: ListBox,
    commits_hashes_list: ListBox,
    commits: Vec<(CommitObject, Option<String>)>,
) {
    let mut hash_sons: HashMap<String, Vec<(f64, f64)>> = HashMap::new(); // hash, Vec<(x,y)> de los hijos
    let mut hash_branches: HashMap<String, usize> = HashMap::new();
    //let mut identado: usize = 1;

    for (mut commit, branch) in commits {
        // let y = add_row_to_list(&commit.get_message(), &description_list);
        //identado = make_graph(
        //     &drawing_area,
        //     &mut hash_branches,
        //     &mut hash_sons,
        //     &mut identado,
        //     &commit_and_branches,
        //     y,
        // );
        add_row_to_list(&commit.get_timestamp().to_string(), &date_list);
        add_row_to_list(&commit.get_author(), &author_list);
        add_row_to_list(&commit.get_hash_string().unwrap(), &commits_hashes_list);
    }
}

fn add_row_to_list(row_information: &String, row_list: &ListBox) -> i32 {
    let label = Label::new(Some(&row_information));
    let row_date = ListBoxRow::new();
    row_date.add(&label);
    row_list.add(&row_date);
    row_date.allocation().y()
}

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
