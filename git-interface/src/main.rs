extern crate gtk;
use std::{
    cell::RefCell,
    collections::HashSet,
    io::{self, Write},
    ops::ControlFlow,
    rc::Rc,
};

use gtk::{prelude::*, Button, Label, ListBox, ListBoxRow, Orientation, Window, WindowType};

use git::commands::push::Push;
use git_lib::{
    command_errors::CommandError,
    git_repository::GitRepository,
    objects::{commit_object::CommitObject, git_object::GitObjectTrait},
};

// colores para el grafo en el futuro
const _GRAPH_COLORS: [(f64, f64, f64); 10] = [
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
    window: Rc<RefCell<gtk::Window>>,
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let repo_dir_text = "".to_string();
    let glade_src = include_str!("../git interface.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let inicial_window: gtk::Window = builder.object("inicial_window").unwrap();
    inicial_window.show_all();

    let inicial_apply: gtk::Button = builder.object("apply_button_inicial").unwrap();
    let repo_dir: gtk::Entry = builder.object("entry_for_inicial").unwrap();
    let correct_path = false;

    let rc_repo_dir_text = Rc::new(RefCell::new(repo_dir_text));
    let rc_correct_path = Rc::new(RefCell::new(correct_path));
    let rc_builder = Rc::new(RefCell::new(builder));

    let clone_rc_repo_dir_text = rc_repo_dir_text.clone();

    ventana_inicial(
        inicial_apply,
        rc_correct_path,
        clone_rc_repo_dir_text,
        inicial_window,
        repo_dir,
    );
    git_interface(rc_repo_dir_text.borrow_mut().to_string(), rc_builder);
    gtk::main();
}

fn ventana_inicial(
    inicial_apply: Button,
    rc_correct_path: Rc<RefCell<bool>>,
    clone_rc_repo_dir_text: Rc<RefCell<String>>,
    inicial_window: Window,
    repo_dir: gtk::Entry,
) {
    let clone_rc_correct_path = rc_correct_path.clone();
    inicial_apply.connect_clicked(move |_| {
        let clone_correct_path_clone = clone_rc_correct_path.clone();
        let repo_dir_text_clone = clone_rc_repo_dir_text.clone();
        // let inicial_window = inicial_window.clone();
        let repo_dir = repo_dir.clone();

        let repo_dir = repo_dir.clone();
        let repo_dir_text = repo_dir.text().to_string();
        println!("repo_dir_text: {:?}", repo_dir_text);
        // let inicial_window = inicial_window.clone();
        let mut binding = io::stdout();
        if GitRepository::open(&repo_dir_text, &mut binding).is_err() {
            repo_dir.set_text("");
            dialog_window(
                format!(
                    "No se pudo conectar satisfactoriamente a un repositorio Git en {}",
                    repo_dir_text_clone.borrow_mut()
                )
                .to_string(),
            );
        } else {
            *clone_correct_path_clone.borrow_mut() = true;
            inicial_window.hide();
            gtk::main_quit();
        }
    });
    gtk::main();
}

fn git_interface(repo_git_path: String, builder: Rc<RefCell<gtk::Builder>>) -> ControlFlow<()> {
    let mut output = io::stdout();
    let mut repo = match GitRepository::open(&repo_git_path, &mut output) {
        Ok(repo) => repo,
        Err(_) => {
            eprintln!("No se pudo conectar satisfactoriamente a un repositorio Git.");
            return ControlFlow::Break(());
        }
    };
    let (staging_changes, unstaging_changes) = staged_area_func(repo_git_path.to_string()).unwrap();
    let window: gtk::Window = builder.borrow_mut().object("window app").unwrap();

    let builder_interface = builder.borrow_mut().clone();
    let mut interface = Interface {
        builder: builder_interface,
        repo_git_path,
        staging_changes: Rc::new(RefCell::new(staging_changes)),
        unstaging_changes: Rc::new(RefCell::new(unstaging_changes)),
        window: Rc::new(RefCell::new(window)),
    };
    let commits = match repo.get_log(true) {
        Ok(commits) => commits,
        Err(err) => {
            dialog_window(err.to_string());
            return ControlFlow::Break(());
        }
    };
    interface.staged_area_ui();
    let err_activation = interface.buttons_activation();
    if err_activation.is_err() {
        dialog_window(err_activation.unwrap_err().to_string());
        return ControlFlow::Break(());
    }
    interface.set_right_area(commits);
    interface.window.borrow_mut().connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    interface.window.borrow_mut().show_all();

    ControlFlow::Continue(())
}

impl Interface {
    fn actualizar(&mut self) -> Option<Vec<(CommitObject, Option<String>)>> {
        let (staging_changes, unstaging_changes) =
            staged_area_func(self.repo_git_path.to_string()).unwrap();
        self.staging_changes = Rc::new(RefCell::new(staging_changes));
        self.unstaging_changes = Rc::new(RefCell::new(unstaging_changes));

        let mut binding = io::stdout();
        let mut repo = match GitRepository::open(&self.repo_git_path, &mut binding) {
            Ok(repo) => repo,
            Err(error) => {
                dialog_window(error.to_string());
                return None;
            }
        };

        let commits = match repo.get_log(true) {
            Ok(commits) => commits,
            Err(error) => {
                dialog_window(error.to_string());
                return None;
            }
        };
        Some(commits)
    }

    fn buttons_activation<'a>(&mut self) -> Result<(), CommandError> {
        let buttons = [
            ("pull", self.build_button("pull_button".to_string())),
            ("push", self.build_button("push_button".to_string())),
            ("checkout", self.build_button("checkout_button".to_string())),
            ("fetch", self.build_button("fetch_button".to_string())),
            ("branch", self.build_button("branch_button".to_string())),
            ("commit", self.build_button("commit_button".to_string())),
            ("refresh", self.build_button("refresh_button".to_string())),
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
        let clone_builder = self.builder.clone();
        let unstaging_changes = Rc::clone(&self.unstaging_changes);
        let staging_changes = Rc::clone(&self.staging_changes);
        let window = self.window.clone();

        button.connect_clicked(move |_| {
            let window = window.clone();
            let builder = clone_builder.clone();
            let output = io::stdout();
            let mut binding = &output;

            let mut repo = match GitRepository::open(&repo_git_path, &mut binding) {
                Ok(repo) => repo,
                Err(_) => {
                    dialog_window(
                        "No se pudo conectar satisfactoriamente a un repositorio Git.".to_string(),
                    );
                    window.borrow_mut().hide();
                    return;
                }
            };

            match button_action.as_str() {
                "pull" => {
                    let err = repo.pull();
                    let mut message_for_pull =
                        "Realice refresh para obtener los cambios".to_string();
                    if err.is_err() {
                        let err = err.unwrap_err();
                        message_for_pull = err.to_string() + "\nPull no pudo realizarse con éxito";
                    }
                    dialog_window(message_for_pull);
                }
                "push" => {
                    let mut binding_for_push = &output;
                    let result_for_push = push_function(&mut binding_for_push);
                    if result_for_push.is_err() {
                        dialog_window(result_for_push.unwrap_err().to_string());
                        return;
                    }
                }
                "fetch" => {
                    if let Err(err) = repo.fetch() {
                        dialog_window(err.to_string());
                        return;
                    }
                    dialog_window("Fetch realizado con éxito".to_string());
                }
                "branch" => {
                    let mut interface = Interface {
                        builder: builder.clone(),
                        repo_git_path: repo_git_path.to_string(),
                        staging_changes: Rc::clone(&staging_changes),
                        unstaging_changes: Rc::clone(&unstaging_changes),
                        window,
                    };
                    interface.branch_function();
                }
                "commit" => {
                    commit_function(&mut repo, builder);
                }
                "refresh" => {
                    let mut interface = Interface {
                        builder: builder.clone(),
                        repo_git_path: repo_git_path.to_string(),
                        staging_changes: Rc::clone(&staging_changes),
                        unstaging_changes: Rc::clone(&unstaging_changes),
                        window,
                    };
                    interface.staged_area_ui();
                    let commits = match interface.actualizar() {
                        Some(commits) => commits,
                        None => return,
                    };
                    interface.set_right_area(commits);
                    interface.staged_area_ui();
                }
                "checkout" => {
                    todo!();
                }
                _ => {
                    eprintln!("Acción no reconocida: {}", button_action);
                }
            }
        });
        Ok(())
    }

    fn staged_area_ui(&self) {
        let staging_changes: gtk::ListBox = self.builder.object("staging_list").unwrap();
        let unstaging_changes: gtk::ListBox = self.builder.object("unstaging_list").unwrap();

        remove_childs(&staging_changes);
        remove_childs(&unstaging_changes);

        self.stage_and_unstage_ui(unstaging_changes, self.unstaging_changes.clone(), true);
        self.stage_and_unstage_ui(staging_changes, self.staging_changes.clone(), false);
    }

    fn stage_and_unstage_ui(
        &self,
        list_box: ListBox,
        files: Rc<RefCell<HashSet<String>>>,
        is_unstaging: bool,
    ) {
        for file in files.borrow_mut().iter() {
            let file = file.clone();
            let box_outer = gtk::Box::new(Orientation::Horizontal, 0);

            let mut button = Button::with_label("stage");
            if !is_unstaging {
                button = Button::with_label("unstage");
            }
            let label = Label::new(Some(&format!("{}", file)));

            box_outer.pack_start(&label, true, true, 0);
            box_outer.pack_end(&button, false, false, 0);

            list_box.add(&box_outer);

            let window = self.window.clone();
            let builder = self.builder.clone();
            let staging_changes = Rc::clone(&self.staging_changes);
            let unstaging_changes = Rc::clone(&self.unstaging_changes);

            self.window.borrow_mut().show_all();

            let repo_git_path = self.repo_git_path.clone();
            button.connect_clicked(move |_| {
                let mut binding = io::stdout();
                let mut repo = GitRepository::open(&repo_git_path, &mut binding).unwrap();
                let vec_files = vec![file.clone()];

                if is_unstaging {
                    _ = unstaging_changes.borrow_mut().take(&file);
                    staging_changes.borrow_mut().insert(file.clone());
                    let err = repo.add(vec_files);
                    if err.is_err() {
                        dialog_window(err.unwrap_err().to_string());
                        return;
                    }
                    println!("unstaged files: {:?}", unstaging_changes.borrow());
                } else {
                    _ = staging_changes.borrow_mut().take(&file);
                    unstaging_changes.borrow_mut().insert(file.clone());
                    repo.remove_cached(vec_files).unwrap();
                    println!("staged files: {:?}", unstaging_changes.borrow());
                }
                let interface = Interface {
                    builder: builder.clone(),
                    repo_git_path: repo_git_path.to_string(),
                    staging_changes: Rc::clone(&staging_changes),
                    unstaging_changes: Rc::clone(&unstaging_changes),
                    window: window.clone(),
                };
                interface.staged_area_ui();
            });
        }
    }

    fn set_right_area(&mut self, commits: Vec<(CommitObject, Option<String>)>) {
        let date_list: gtk::ListBox = self.builder.object("date_list").unwrap();
        let author_list: gtk::ListBox = self.builder.object("author_list").unwrap();
        let drawing_area: gtk::DrawingArea = self.builder.object("drawing_area").unwrap();
        let _stagin_changes_list: gtk::ListBox = self.builder.object("staging_list").unwrap();
        let description_list: gtk::ListBox = self.builder.object("description_list").unwrap();
        let commits_hashes_list: gtk::ListBox = self.builder.object("commit_hash_list").unwrap();

        remove_childs(&description_list);
        remove_childs(&date_list);
        remove_childs(&author_list);
        remove_childs(&commits_hashes_list);

        // let hash_sons: HashMap<String, Vec<(f64, f64)>> = HashMap::new(); // hash, Vec<(x,y)> de los hijos
        // let hash_branches: HashMap<String, usize> = HashMap::new();

        for (mut commit, branch) in commits {
            add_row_to_list(&commit.get_timestamp_string(), &date_list);
            add_row_to_list(&commit.get_author(), &author_list);
            add_row_to_list(&commit.get_hash_string().unwrap(), &commits_hashes_list);
            add_row_to_list(&commit.get_message(), &description_list);
        }
        self.window.borrow_mut().show_all();
    }

    fn branch_function(&mut self) {
        let branch_window: gtk::Window = self.builder.object("branch_window").unwrap();
        let branches_list: gtk::ListBox = self.builder.object("branches_list").unwrap();
        let apply_button: gtk::Button = self.builder.object("apply_button").unwrap();
        let mut binding = io::stdout();
        let Ok(mut repo) = GitRepository::open(&self.repo_git_path, &mut binding) else {
            dialog_window(
                "No se pudo conectar satisfactoriamente a un repositorio Git.".to_string(),
            );
            return;
        };
        let local_branches = match repo.local_branches() {
            Ok(local_branches) => local_branches,
            Err(err) => {
                dialog_window(err.to_string());
                return;
            }
        };
        for branch in &local_branches {
            add_row_to_list(&branch.0, &branches_list);
        }
        branch_window.show_all();
        let name_branch: gtk::Entry = self.builder.object("entry_for_new_branch").unwrap();

        let repo_git_path = self.repo_git_path.clone();
        apply_button.connect_clicked(move |_| {
            let mut binding = io::stdout();
            let Ok(mut repo) = GitRepository::open(&repo_git_path, &mut binding) else {
                dialog_window(
                    "No se pudo conectar satisfactoriamente a un repositorio Git.".to_string(),
                );
                return;
            };
            let name_branch_text = name_branch.text();
            if name_branch_text.is_empty() {
                dialog_window("No se ha ingresado un nombre para la rama".to_string());
                return;
            }
            let vec_branch = vec![name_branch_text.to_string()];
            println!("vec_branch: {:?}", vec_branch);
            match repo.create_branch(&vec_branch) {
                Ok(_) => dialog_window("Rama creada con éxito".to_string()),
                Err(err) => dialog_window(err.to_string()),
            };
            remove_childs(&branches_list);
            branch_window.hide();
        });
    }
}

fn staged_area_func(
    repo_git_path: String,
) -> Result<(HashSet<String>, HashSet<String>), CommandError> {
    let mut output = io::stdout();
    let mut repo = GitRepository::open(&repo_git_path, &mut output).unwrap();
    repo.get_stage_and_unstage_changes()
}

fn commit_function(repo: &mut GitRepository, builder: gtk::Builder) {
    let commit_entry_msg: gtk::Entry = builder
        .object("entrada_de_mensaje")
        .expect("No se pudo obtener la entrada de mensaje");
    let message: gtk::glib::GString = commit_entry_msg.text();

    if message.is_empty() {
        dialog_window("No se ha ingresado un mensaje de commit".to_string());
        return;
    }

    commit_entry_msg.set_text("");

    match repo.commit(message.to_string(), &vec![], false, None, false) {
        Ok(_) => dialog_window(
            "Commit realizado con éxito\nRealice refresh para ver los cambios".to_string(),
        ),
        Err(err) => dialog_window(err.to_string()),
    };
}

fn dialog_window(message: String) {
    let window = Window::new(WindowType::Toplevel);
    window.set_title(&message);
    window.set_default_size(300, 200);

    let dialog = gtk::MessageDialog::new(
        Some(&window),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Info,
        gtk::ButtonsType::Close,
        &message,
    );

    dialog.connect_response(|dialog, _| {
        dialog.hide();
    });

    dialog.run();
}

fn push_function(output: &mut dyn Write) -> Result<(), CommandError> {
    let push = Push::new_default(output).unwrap();
    push.run(output)
}

fn remove_childs(list: &ListBox) {
    list.foreach(|child| {
        list.remove(child);
    });
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
