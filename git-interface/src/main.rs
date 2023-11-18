extern crate gtk;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::{self, BufRead, Write},
    ops::ControlFlow,
    rc::Rc,
};

use gtk::{
    prelude::*, Button, DrawingArea, Label, ListBox, ListBoxRow, Orientation, Window, WindowType,
};

use git::commands::push::Push;
use git_lib::{
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
    files_merge_conflict: Rc<RefCell<HashSet<String>>>,
    principal_window: Rc<RefCell<gtk::Window>>,
    // branch_window: Rc<RefCell<gtk::Window>>,
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let repo_dir_text = "".to_string();
    let glade_src = include_str!("../git_interface.glade");
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
    let (staging_changes, unstaging_changes, files_merge_conflict) =
        staged_area_func(repo_git_path.to_string()).unwrap();
    let window: gtk::Window = builder.borrow_mut().object("window app").unwrap();

    let builder_interface = builder.borrow_mut().clone();
    let mut interface = Interface {
        builder: builder_interface,
        repo_git_path,
        staging_changes: Rc::new(RefCell::new(staging_changes)),
        unstaging_changes: Rc::new(RefCell::new(unstaging_changes)),
        files_merge_conflict: Rc::new(RefCell::new(files_merge_conflict)),
        principal_window: Rc::new(RefCell::new(window)),
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
    interface.set_right_area(&commits);
    interface
        .principal_window
        .borrow_mut()
        .connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });
    interface.principal_window.borrow_mut().show_all();

    ControlFlow::Continue(())
}

impl Interface {
    fn actualizar(&mut self) -> Option<Vec<(CommitObject, Option<String>)>> {
        let (staging_changes, unstaging_changes, merge_conflicts) =
            staged_area_func(self.repo_git_path.to_string()).unwrap();
        self.staging_changes = Rc::new(RefCell::new(staging_changes));
        self.unstaging_changes = Rc::new(RefCell::new(unstaging_changes));
        self.files_merge_conflict = Rc::new(RefCell::new(merge_conflicts));

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
        let files_merge_conflict = Rc::clone(&self.files_merge_conflict);
        let window = self.principal_window.clone();

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
                        files_merge_conflict: Rc::clone(&files_merge_conflict),
                        principal_window: window,
                    };
                    interface.branch_function();
                }
                "commit" => {
                    commit_function(&mut repo, builder);
                }
                "refresh" => {
                    let interface = Interface {
                        builder: builder.clone(),
                        repo_git_path: repo_git_path.to_string(),
                        staging_changes: Rc::clone(&staging_changes),
                        unstaging_changes: Rc::clone(&unstaging_changes),
                        files_merge_conflict: Rc::clone(&files_merge_conflict),
                        principal_window: window,
                    };
                    if let ControlFlow::Break(_) = refresh_function(interface) {
                        return;
                    }
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
        let merge_conflicts: gtk::ListBox = self.builder.object("merge_conflicts_list").unwrap();

        remove_childs(&staging_changes);
        remove_childs(&unstaging_changes);
        remove_childs(&merge_conflicts);

        self.stage_and_unstage_ui(
            unstaging_changes,
            self.unstaging_changes.clone(),
            "unstaging".to_string(),
        );
        self.stage_and_unstage_ui(
            staging_changes,
            self.staging_changes.clone(),
            "staging".to_string(),
        );
        self.stage_and_unstage_ui(
            merge_conflicts,
            self.files_merge_conflict.clone(),
            "merge".to_string(),
        );
    }

    fn stage_and_unstage_ui(
        &self,
        list_box: ListBox,
        files: Rc<RefCell<HashSet<String>>>,
        field: String,
    ) {
        let clone_field = field.clone();

        for file in files.borrow_mut().iter() {
            let file = file.clone();
            let field2 = clone_field.clone();
            let window = self.principal_window.clone();
            let window2 = self.principal_window.clone();
            let builder = self.builder.clone();
            let builder2 = self.builder.clone();
            let staging_changes = Rc::clone(&self.staging_changes);
            let staging_changes2 = Rc::clone(&self.staging_changes);
            let unstaging_changes = Rc::clone(&self.unstaging_changes);
            let unstaging_changes2 = Rc::clone(&self.unstaging_changes);
            let files_merge_conflict = Rc::clone(&self.files_merge_conflict);
            let repo_git_path = self.repo_git_path.clone();

            let box_outer = gtk::Box::new(Orientation::Horizontal, 0);
            let mut button = Button::with_label("stage");

            match field2.as_str() {
                "staging" => {
                    button = Button::with_label("unstage");
                }
                "merge" => {
                    let add_button = Button::with_label("resolve");
                    box_outer.pack_end(&add_button, false, false, 0);
                    let repo_git_path_clone = repo_git_path.clone();

                    let files_merge_conflict_clone = Rc::clone(&files_merge_conflict);

                    let clone_file = file.clone();
                    add_button.connect_clicked(move |_| {
                        let clone_file = clone_file.clone();
                        let mut interface = Interface {
                            builder: builder.clone(),
                            repo_git_path: repo_git_path_clone.to_string(),
                            staging_changes: Rc::clone(&staging_changes),
                            unstaging_changes: Rc::clone(&unstaging_changes),
                            files_merge_conflict: Rc::clone(&files_merge_conflict_clone),
                            principal_window: window.clone(),
                        };
                        println!("inicialize");
                        interface.inicialize(clone_file);
                    });
                }
                _ => {}
            }
            let label = Label::new(Some(&format!("{}", file)));

            box_outer.pack_start(&label, true, true, 0);
            box_outer.pack_end(&button, false, false, 0);

            list_box.add(&box_outer);

            self.principal_window.borrow_mut().show_all();

            let files_merge_conflict = files_merge_conflict.clone();

            let clone_file = file.clone();
            button.connect_clicked(move |_| {
                let mut binding = io::stdout();
                let mut repo = GitRepository::open(&repo_git_path, &mut binding).unwrap();
                let vec_files = vec![file.clone()];

                match field2.clone().as_str() {
                    "unstaging" => {
                        _ = unstaging_changes2.borrow_mut().take(&clone_file);
                        staging_changes2.borrow_mut().insert(file.clone());
                        let err = repo.add(vec_files);
                        if err.is_err() {
                            dialog_window(err.unwrap_err().to_string());
                            return;
                        }
                    }
                    "merge" => {
                        _ = files_merge_conflict.borrow_mut().take(&clone_file);
                        staging_changes2.borrow_mut().insert(file.clone());
                        let err = repo.add(vec_files);
                        if err.is_err() {
                            dialog_window(err.unwrap_err().to_string());
                            return;
                        }
                    }
                    "staging" => {
                        _ = staging_changes2.borrow_mut().take(&clone_file);
                        unstaging_changes2.borrow_mut().insert(file.clone());
                        repo.remove_cached(vec_files).unwrap();
                    }
                    _ => {
                        eprintln!("Acción no reconocida: {}", field2.clone());
                    }
                }

                let interface = Interface {
                    builder: builder2.clone(),
                    repo_git_path: repo_git_path.to_string(),
                    staging_changes: Rc::clone(&staging_changes2),
                    unstaging_changes: Rc::clone(&unstaging_changes2),
                    files_merge_conflict: Rc::clone(&files_merge_conflict),
                    principal_window: window2.clone(),
                };
                interface.staged_area_ui();
            });
        }
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
        let new_name_branch: gtk::Entry = self.builder.object("entry_for_new_branch").unwrap();

        let repo_git_path = self.repo_git_path.clone();
        apply_button.connect_clicked(move |_| {
            let mut binding = io::stdout();
            let Ok(mut repo) = GitRepository::open(&repo_git_path, &mut binding) else {
                dialog_window(
                    "No se pudo conectar satisfactoriamente a un repositorio Git.".to_string(),
                );
                return;
            };
            let name_branch_text = new_name_branch.text();
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

            branch_window.connect_delete_event(|_, _| {
                // Devolver `Inhibit(false)` para cerrar la ventana principal.
                // Devolver `Inhibit(true)` para evitar que la ventana principal se cierre.
                Inhibit(false)
            });

            branch_window.connect_destroy(|_| {
                gtk::main_quit();
            });
        });
    }

    fn set_right_area(&mut self, commits: &Vec<(CommitObject, Option<String>)>) {
        let date_list: gtk::ListBox = self.builder.object("date_list").unwrap();
        let author_list: gtk::ListBox = self.builder.object("author_list").unwrap();
        let drawing_area: gtk::DrawingArea = self.builder.object("drawing_area").unwrap();
        let _stagin_changes_list: gtk::ListBox = self.builder.object("staging_list").unwrap();
        let description_list: gtk::ListBox = self.builder.object("description_list").unwrap();
        let commits_hashes_list: gtk::ListBox = self.builder.object("commit_hash_list").unwrap();

        let children = self.principal_window.to_owned().borrow().children();
        for child in children {
            if child.is::<gtk::DrawingArea>() {
                self.principal_window.to_owned().borrow_mut().remove(&child); // VER COMO REMOVER EL DIBUJO
            }
        }

        drawing_area.queue_draw();

        remove_childs(&description_list);
        remove_childs(&date_list);
        remove_childs(&author_list);
        remove_childs(&commits_hashes_list);

        let mut hash_sons: HashMap<String, Vec<(f64, f64, String)>> = HashMap::new(); // hash, Vec<(x,y)> de los hijos
        let mut hash_branches: HashMap<String, usize> = HashMap::new();
        let mut identado: usize = 1;
        let mut y = 11;

        for commit_and_branches in commits {
            let mut commit = commit_and_branches.0.to_owned();
            add_row_to_list(&commit.get_message(), &description_list);
            add_row_to_list(&commit.get_timestamp_string(), &date_list);
            add_row_to_list(&commit.get_author(), &author_list);
            add_row_to_list(&commit.get_hash_string().unwrap(), &commits_hashes_list);

            identado = make_graph(
                &drawing_area,
                &mut hash_branches,
                &mut hash_sons,
                &mut identado,
                &commit_and_branches,
                y,
            );
            y += 20;
        }

        self.principal_window.borrow_mut().show_all();
    }

    fn inicialize(&mut self, path_file: String) {
        let mut new_content = Rc::new(RefCell::new(String::new()));
        let mut current_content = Rc::new(RefCell::new(String::new()));
        let mut incoming_content = Rc::new(RefCell::new(String::new()));
        let mut old_content = Rc::new(RefCell::new(String::new()));

        let mut binding = io::stdout();
        let repo = GitRepository::open(&self.repo_git_path, &mut binding).unwrap();
        let mut reader = repo.get_file_reader(path_file.to_string()).unwrap();

        let mut line = String::new();

        while let Ok(byte) = reader.read_line(&mut line) {
            if byte == 0 {
                break;
            }
            old_content.borrow_mut().push_str(&line);
            old_content.borrow_mut().push('\n');
            line = String::new();
        }

        actualize_conflicts(
            "current",
            &mut new_content,
            &mut current_content,
            &mut incoming_content,
            &mut old_content,
        );

        let merge_window: gtk::Window = self.builder.object("merge_window").unwrap();
        merge_window.show_all();

        let buttons = ["current", "incoming", "next"];

        for button in buttons {
            self.function_button(
                button,
                &new_content,
                &current_content,
                &incoming_content,
                &old_content,
            );
        }

        let finalize_conflict_button: gtk::Button =
            self.builder.object("finalize_conflict_button").unwrap();

        let new_content_clone = new_content.clone();
        let current_content_clone = current_content.clone();
        let incoming_content_clone = incoming_content.clone();
        let old_content_clone = old_content.clone();
        let repo_git_path = self.repo_git_path.clone();

        let label_contents = [
            ("new", new_content),
            ("current", current_content),
            ("incoming", incoming_content),
            ("old", old_content),
        ];

        for (label_name, content) in label_contents {
            let mut builder = self.builder.clone();

            actualize_label(&mut builder, label_name, &content);
        }

        let builder = self.builder.clone();
        let window = self.principal_window.clone();
        let staging_changes = self.staging_changes.clone();
        let unstaging_changes = self.unstaging_changes.clone();
        let files_merge_conflict = self.files_merge_conflict.clone();

        finalize_conflict_button.connect_clicked(move |_| {
            let new_content_clone = new_content_clone.clone();
            let current_content_clone = current_content_clone.clone();
            let incoming_content_clone = incoming_content_clone.clone();
            let old_content_clone = old_content_clone.clone();
            let repo_git_path = repo_git_path.clone();

            // actualizamos el contenido de new_content con los contenidos restantes de los otros
            new_content_clone
                .borrow_mut()
                .push_str(&current_content_clone.borrow_mut());
            new_content_clone
                .borrow_mut()
                .push_str(&incoming_content_clone.borrow_mut());
            new_content_clone
                .borrow_mut()
                .push_str(&old_content_clone.borrow_mut());

            let mut binding = io::stdout();
            let mut repo = GitRepository::open(&repo_git_path, &mut binding).unwrap();
            repo.write_file(&path_file, &mut new_content_clone.borrow_mut())
                .unwrap();

            let mut staging_area = repo.staging_area().unwrap();
            staging_area.remove_from_unmerged_files(&path_file);

            let interface = Interface {
                builder: builder.clone(),
                repo_git_path: repo_git_path.to_string(),
                staging_changes: Rc::clone(&staging_changes),
                unstaging_changes: Rc::clone(&unstaging_changes),
                files_merge_conflict: Rc::clone(&files_merge_conflict),
                principal_window: window.clone(),
            };
            refresh_function(interface);

            merge_window.hide();
        });
    }

    fn function_button(
        &mut self,
        button_name: &str,
        new_content: &Rc<RefCell<String>>,
        current_content: &Rc<RefCell<String>>,
        incoming_content: &Rc<RefCell<String>>,
        old_content: &Rc<RefCell<String>>,
    ) {
        let button_str = format!("accept_{}_button", button_name);
        let accept_button: gtk::Button = self.builder.object(&button_str).unwrap();

        let new_content_clone = new_content.clone();
        let current_content_clone = current_content.clone();
        let incoming_content_clone = incoming_content.clone();
        let old_content_clone = old_content.clone();

        let button_name = button_name.to_string();
        let builder = self.builder.clone();

        accept_button.connect_clicked(move |_| {
            let mut new_content_clone = new_content_clone.clone();
            let mut current_content_clone = current_content_clone.clone();
            let mut incoming_content_clone = incoming_content_clone.clone();
            let mut old_content_clone = old_content_clone.clone();

            actualize_conflicts(
                &button_name,
                &mut new_content_clone,
                &mut current_content_clone,
                &mut incoming_content_clone,
                &mut old_content_clone,
            );

            let label_contents = [
                ("new", new_content_clone),
                ("current", current_content_clone),
                ("incoming", incoming_content_clone),
                ("old", old_content_clone),
            ];

            for (label_name, content) in label_contents {
                let mut builder = builder.clone();
                actualize_label(&mut builder, label_name, &content);
            }
        });
    }
}

fn refresh_function(mut interface: Interface) -> ControlFlow<()> {
    interface.staged_area_ui();
    let commits = match interface.actualizar() {
        Some(commits) => commits,
        None => return ControlFlow::Break(()),
    };
    interface.set_right_area(&commits);
    interface.staged_area_ui();
    ControlFlow::Continue(())
}

fn actualize_label(builder: &mut gtk::Builder, label_name: &str, content: &Rc<RefCell<String>>) {
    let label_name = format!("{}_content_label", label_name);
    let content_label: gtk::Label = builder.object(&label_name).unwrap();
    content_label.set_text(&content.borrow().to_string());
}

fn staged_area_func(
    repo_git_path: String,
) -> Result<(HashSet<String>, HashSet<String>, HashSet<String>), CommandError> {
    let mut output = io::stdout();
    let mut repo = GitRepository::open(&repo_git_path, &mut output).unwrap();
    let (staging_changes, mut unstaging_changes) = repo.get_stage_and_unstage_changes()?;
    // let staging_area = repo.staging_area()?;
    //let merge_conflicts = staging_area.get_unmerged_files();
    //let files_merge_conflict = merge_conflicts.keys().cloned().collect::<HashSet<String>>();

    let mut files_merge_conflict = HashSet::new();
    files_merge_conflict.insert("meli.txt".to_string());
    files_merge_conflict.insert("ian.txt".to_string());

    //ELIMINARRRRRRR
    for conflict in &staging_changes {
        files_merge_conflict.remove(conflict);
        println!("conflict: {}", conflict);
    }

    for conflict in &files_merge_conflict {
        unstaging_changes.remove(conflict);
    }
    Ok((staging_changes, unstaging_changes, files_merge_conflict))
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

fn add_row_to_list(row_information: &String, row_list: &ListBox) {
    let label = Label::new(Some(&row_information));
    let row_date = ListBoxRow::new();
    row_date.add(&label);
    row_list.add(&row_date);
}

fn make_graph(
    drawing_area: &DrawingArea,
    hash_branches: &mut HashMap<String, usize>,
    hash_sons: &mut HashMap<String, Vec<(f64, f64, String)>>,
    identado: &mut usize,
    commit: &(CommitObject, Option<String>),
    y: i32,
) -> usize {
    let commit_branch = commit.1.as_ref().unwrap();
    //let commit_obj = &commit.0;
    if !hash_branches.contains_key(commit_branch) {
        hash_branches.insert(commit_branch.clone(), *identado);
        *identado += 1;
    }

    let i = hash_branches.get(commit_branch).unwrap();
    let index_color = i % GRAPH_COLORS.len();
    let (c1, c2, c3): (f64, f64, f64) = GRAPH_COLORS[index_color];
    let x: f64 = *i as f64 * 30.0;
    // Conéctate al evento "draw" del DrawingArea para dibujar
    draw_commit_point(drawing_area, c1, c2, c3, x, y as f64);

    let commit_hash = commit.0.to_owned().get_hash_string().unwrap();

    draw_lines_to_sons(
        hash_sons,
        &commit_hash,
        drawing_area,
        hash_branches,
        x,
        y as f64,
    );

    for parent in &commit.0.get_parents() {
        let sons_parent = hash_sons.entry(parent.clone()).or_default();
        sons_parent.push((x, y as f64, commit_branch.clone()));
    }

    return *identado;
}

fn draw_lines_to_sons(
    hash_sons: &mut HashMap<String, Vec<(f64, f64, String)>>,
    commit_hash: &String,
    drawing_area: &DrawingArea,
    hash_branches: &mut HashMap<String, usize>,
    // c1: f64,
    // c2: f64,
    // c3: f64,
    x: f64,
    y: f64,
) {
    if hash_sons.contains_key(commit_hash) {
        for sons in hash_sons.get(commit_hash).unwrap() {
            let sons_clone = sons.clone();
            let i = hash_branches.get(&sons.2).unwrap();
            let index_color = i % GRAPH_COLORS.len();
            let (c1, c2, c3): (f64, f64, f64) = GRAPH_COLORS[index_color];

            drawing_area.connect_draw(move |_, context| {
                // Dibuja una línea en el DrawingArea
                context.set_source_rgb(c1, c2, c3);
                context.set_line_width(5.0);
                context.move_to(x, y);
                context.line_to(sons_clone.0.clone(), sons_clone.1.clone());
                context.stroke().unwrap();
                Inhibit(false)
            });
        }
    }
}

fn draw_commit_point(drawing_area: &DrawingArea, c1: f64, c2: f64, c3: f64, x: f64, y: f64) {
    drawing_area.connect_draw(move |_, context| {
        // Dibuja un punto en la posición (100, 100)
        context.set_source_rgb(c1, c2, c3); // Establece el color en rojo
        context.arc(x, y, 5.0, 0.0, 2.0 * std::f64::consts::PI); // Dibuja un círculo (punto)
        context.fill().unwrap();
        Inhibit(false)
    });
}

fn actualize_conflicts(
    accept_changes: &str,
    new_content: &mut Rc<RefCell<String>>,
    current_content: &mut Rc<RefCell<String>>,
    incoming_content: &mut Rc<RefCell<String>>,
    old_content: &mut Rc<RefCell<String>>,
) {
    let mut found_next_conflict = false;

    if accept_changes == "current" {
        new_content
            .borrow_mut()
            .push_str(current_content.borrow().as_str());
    } else if accept_changes == "incoming" {
        new_content
            .borrow_mut()
            .push_str(incoming_content.borrow().as_str());
    } else {
        new_content
            .borrow_mut()
            .push_str(current_content.borrow().as_str());
        new_content
            .borrow_mut()
            .push_str(incoming_content.borrow().as_str());
    }

    let mut old_new_content = String::new();
    current_content.borrow_mut().clear();
    incoming_content.borrow_mut().clear();

    let binding = old_content.borrow_mut().clone();
    let mut lines = binding.lines();

    while let Some(line) = lines.next() {
        if line.starts_with("<<<<<<<") && !found_next_conflict {
            found_next_conflict = true;
            while let Some(current_line) = lines.next() {
                if current_line.starts_with("=======") {
                    break;
                }
                current_content.borrow_mut().push_str(current_line);
                current_content.borrow_mut().push('\n');
            }

            while let Some(inner_line) = lines.next() {
                if inner_line.starts_with(">>>>>>>") {
                    break;
                }
                incoming_content.borrow_mut().push_str(inner_line);
                incoming_content.borrow_mut().push('\n');
            }
            continue;
        }

        if !found_next_conflict {
            new_content.borrow_mut().push_str(line);
            new_content.borrow_mut().push('\n');
        } else {
            old_new_content.push_str(line);
            old_new_content.push('\n');
        }
    }

    old_content.borrow_mut().clear();
    old_content.borrow_mut().push_str(old_new_content.as_str());
}
