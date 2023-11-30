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
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let repo_dir_text = "".to_string();
    let glade_src = include_str!("../git_interface.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let chooser_window: gtk::FileChooserDialog = builder.object("choose_directory").unwrap();
    chooser_window.show_all();

    let connect_choose_button: gtk::Button = builder.object("connect_choose_button").unwrap();
    let cancel_choose_button: gtk::Button = builder.object("cancel_choose_button").unwrap();
    let correct_path = false;

    let rc_repo_dir_text = Rc::new(RefCell::new(repo_dir_text));
    let rc_correct_path = Rc::new(RefCell::new(correct_path));
    let rc_builder = Rc::new(RefCell::new(builder));

    initial_window(
        connect_choose_button,
        cancel_choose_button,
        rc_correct_path.clone(),
        rc_repo_dir_text.clone(),
        chooser_window,
    );

    if rc_correct_path.borrow_mut().to_owned() == false {
        return;
    }

    git_interface(rc_repo_dir_text.borrow_mut().to_string(), rc_builder);
    gtk::main();
}

fn initial_window(
    inicial_apply: Button,
    cancel_choose_button: Button,
    rc_correct_path: Rc<RefCell<bool>>,
    rc_repo_dir_text: Rc<RefCell<String>>,
    chooser_dialog_window: gtk::FileChooserDialog,
) {
    let chooser_dialog_window_clone = chooser_dialog_window.clone();
    inicial_apply.connect_clicked(move |_| {
        if let Some(file) = chooser_dialog_window_clone.current_folder() {
            let repo_dir_text = file.to_str().unwrap().to_string();
            let mut binding = io::stdout();
            if GitRepository::open(&repo_dir_text, &mut binding).is_err() {
                *rc_repo_dir_text.borrow_mut() = file.to_str().unwrap().to_string();
                dialog_window(
                    format!(
                        "No se pudo conectar satisfactoriamente a un repositorio Git en {}",
                        rc_repo_dir_text.borrow_mut()
                    )
                    .to_string(),
                );
            } else {
                *rc_correct_path.borrow_mut() = true;
                *rc_repo_dir_text.borrow_mut() = repo_dir_text;
                chooser_dialog_window_clone.hide();
                gtk::main_quit();
            }
        } else {
            dialog_window("No se ha seleccionado un directorio que proporcione git".to_string());
        }
    });
    chooser_dialog_window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    cancel_choose_button.connect_clicked(move |_| {
        chooser_dialog_window.hide();
        gtk::main_quit();
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

    let mut interface = Interface {
        builder: builder.borrow_mut().clone(),
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
    interface.set_right_area_ui(&commits);
    interface
        .principal_window
        .clone()
        .borrow_mut()
        .connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });
    interface.principal_window.borrow_mut().show_all();
    interface.inicialize_apply_button();

    let finalize_conflict_button: gtk::Button = interface
        .builder
        .object("finalize_conflict_button")
        .unwrap();

    finalize_conflict_button.connect_clicked(move |_| {
        let principal_window = interface.principal_window.clone();
        let merge_window: gtk::Window = interface.builder.object("merge_window").unwrap();
        let new_label: gtk::Label = interface.builder.object("new_content_label").unwrap();
        let old_label: gtk::Label = interface.builder.object("old_content_label").unwrap();
        let current_label: gtk::Label = interface.builder.object("current_content_label").unwrap();
        let incoming_label: gtk::Label =
            interface.builder.object("incoming_content_label").unwrap();

        let mut new_content = new_label.text().to_string();
        let current_content = current_label.text().to_string();
        let incoming_content = incoming_label.text().to_string();
        let old_content = old_label.text().to_string();

        let merge_conflicts_label: gtk::Label =
            interface.builder.object("merge_conflicts").unwrap();
        let mut path_file = merge_conflicts_label.text().to_string();
        path_file = path_file.split(" ").collect::<Vec<&str>>()[3..].join(" ");

        // actualizamos el contenido de new_content con los contenidos restantes de los otros
        new_content.push_str(&current_content);
        new_content.push_str(&incoming_content);
        new_content.push_str(&old_content);

        let repo_git_path = interface.repo_git_path.clone();
        let mut binding = io::stdout();
        let mut repo = GitRepository::open(&repo_git_path.to_string(), &mut binding).unwrap();
        repo.write_file(&path_file, &mut new_content).unwrap();

        let mut staging_area = repo.staging_area().unwrap();
        staging_area.remove_from_unmerged_files(&path_file);

        let (staging_changes, unstaging_changes, files_merge_conflict) =
            staged_area_func(repo_git_path.to_string()).unwrap();

        let interface2 = Interface {
            builder: interface.builder.clone(),
            repo_git_path: repo_git_path.to_string(),
            staging_changes: Rc::new(RefCell::new(staging_changes)),
            unstaging_changes: Rc::new(RefCell::new(unstaging_changes)),
            files_merge_conflict: Rc::new(RefCell::new(files_merge_conflict)),
            principal_window: principal_window,
        };
        merge_window.clone().hide();
        remove_widgets_to_merge_window(&mut interface.builder.clone(), merge_window.clone());
        if let ControlFlow::Break(_) = refresh_function(interface2) {
            return;
        }
    });

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
        let staging_changes = self.builder.object("staging_list").unwrap();
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
        //("staging_changes: {:?}", self.unstaging_changes);
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
        let git_path = self.repo_git_path.clone() + "/";
        let files = files.borrow().to_owned();
        let mut files: Vec<String> = files.into_iter().collect();
        files.sort();

        for file in files {
            let file_for_view = file.replace(&git_path, "");
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
                        interface.add_widgets_to_merge_window();
                        interface.initialize_merge(clone_file);
                    });
                }
                _ => {}
            }

            let label = Label::new(Some(&format!("{}", file_for_view)));
            box_outer.pack_start(&label, true, true, 0);
            box_outer.pack_end(&button, false, false, 0);
            list_box.add(&box_outer);
            self.principal_window.borrow_mut().show_all();
            let files_merge_conflict = files_merge_conflict.clone();

            let clone_file = file.clone();
            button.connect_clicked(move |_| {
                let file = &clone_file;
                let mut binding = io::stdout();
                let mut repo = GitRepository::open(&repo_git_path, &mut binding).unwrap();
                let vec_files = vec![clone_file.clone()];

                match field2.clone().as_str() {
                    "unstaging" => {
                        _ = unstaging_changes2.borrow_mut().take(&clone_file);
                        staging_changes2.borrow_mut().insert(file.clone());
                        let err = repo.add(vec_files, false);
                        if err.is_err() {
                            dialog_window(err.unwrap_err().to_string());
                            return;
                        }
                    }
                    "merge" => {
                        _ = files_merge_conflict.borrow_mut().take(&clone_file);
                        staging_changes2.borrow_mut().insert(file.clone());
                        let err = repo.add(vec_files, false);
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
        // Agregamos los widgets a la ventana de branches
        let branch_window = self.add_widgets_to_branch_window();
        let branches_list: gtk::ListBox = self.builder.object("branches_list").unwrap();

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

        // Eliminamos todos los widgets de la ventana de branches
        let branch_window_clone = branch_window.clone();
        let builder = self.builder.clone();
        branch_window.connect_delete_event(move |_, _| {
            let branches_list = branches_list.clone();
            let builder = builder.clone();
            let branch_window_clone = branch_window_clone.clone();
            remove_childs(&branches_list);
            remove_widgets_to_branch_window(builder, branch_window_clone);
            Inhibit(false)
        });
    }

    fn inicialize_apply_button(&mut self) {
        let repo_git_path = self.repo_git_path.clone();
        let apply_button: gtk::Button = self.builder.object("apply_button").unwrap();
        let new_name_branch: gtk::Entry = self.builder.object("entry_for_new_branch").unwrap();
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
            match repo.create_branch(&vec_branch) {
                Ok(_) => dialog_window("Rama creada con éxito".to_string()),
                Err(err) => dialog_window(err.to_string()),
            };
        });
    }

    fn add_widgets_to_merge_window(&mut self) {
        let merge_window: gtk::Window = self.builder.object("merge_window").unwrap();
        let merge_grid: gtk::Grid = self.builder.object("merge_grid").unwrap();
        let merge_conflicts_label: gtk::Label = self.builder.object("merge_conflicts").unwrap();
        let grid_buttons: gtk::Grid = self.builder.object("grid_buttons").unwrap();

        let accept_current_button: gtk::Button =
            self.builder.object("accept_current_button").unwrap();
        let accept_incoming_button: gtk::Button =
            self.builder.object("accept_incoming_button").unwrap();
        let accept_next_button: gtk::Button = self.builder.object("accept_next_button").unwrap();

        let current_scrolled: gtk::ScrolledWindow =
            self.builder.object("current_scrolled").unwrap();
        let viewport_current: gtk::Viewport = self.builder.object("viewport_current").unwrap();
        let current_label: gtk::Label = self.builder.object("current_content_label").unwrap();

        let old_scrolled: gtk::ScrolledWindow = self.builder.object("old_scrolled").unwrap();
        let viewport_old: gtk::Viewport = self.builder.object("viewport_old").unwrap();
        let old_label: gtk::Label = self.builder.object("old_content_label").unwrap();

        let incoming_scrolled: gtk::ScrolledWindow =
            self.builder.object("incoming_scrolled").unwrap();
        let viewport_incoming: gtk::Viewport = self.builder.object("viewport_incoming").unwrap();
        let incoming_label: gtk::Label = self.builder.object("incoming_content_label").unwrap();

        let new_scrolled: gtk::ScrolledWindow = self.builder.object("new_scrolled").unwrap();
        let view_port_new: gtk::Viewport = self.builder.object("viewport_new").unwrap();
        let new_label: gtk::Label = self.builder.object("new_content_label").unwrap();

        let finalize_conflict_button: gtk::Button =
            self.builder.object("finalize_conflict_button").unwrap();

        grid_buttons.attach(&accept_current_button, 0, 0, 1, 1);
        grid_buttons.attach(&accept_incoming_button, 1, 0, 1, 1);
        grid_buttons.attach(&accept_next_button, 2, 0, 1, 1);

        viewport_current.add(&current_label);
        current_scrolled.add(&viewport_current);

        viewport_old.add(&old_label);
        old_scrolled.add(&viewport_old);

        view_port_new.add(&new_label);
        new_scrolled.add(&view_port_new);

        viewport_incoming.add(&incoming_label);
        incoming_scrolled.add(&viewport_incoming);

        merge_grid.attach(&merge_conflicts_label, 0, 0, 1, 1);
        merge_grid.attach(&new_scrolled, 0, 1, 1, 1);
        merge_grid.attach(&grid_buttons, 0, 2, 1, 1);
        merge_grid.attach(&current_scrolled, 0, 3, 1, 1);
        merge_grid.attach(&incoming_scrolled, 0, 4, 1, 1);
        merge_grid.attach(&old_scrolled, 0, 5, 1, 1);
        merge_grid.attach(&finalize_conflict_button, 0, 6, 1, 1);

        merge_window.add(&merge_grid);
    }

    fn add_widgets_to_branch_window(&mut self) -> gtk::Window {
        let branch_window: gtk::Window = self.builder.object("branch_window").unwrap();
        let branch_window_grid: gtk::Grid = self.builder.object("branch_window_grid").unwrap();
        let entry_grid: gtk::Grid = self.builder.object("entry_grid").unwrap();
        let scrolled_window: gtk::ScrolledWindow = self.builder.object("scrolled_window").unwrap();
        let new_branch_label: gtk::Label = self.builder.object("new_branch_label").unwrap();
        let branch_names: gtk::Label = self.builder.object("branch_names").unwrap();
        let entry_for_new_branch: gtk::Entry = self.builder.object("entry_for_new_branch").unwrap();
        let apply_button: gtk::Button = self.builder.object("apply_button").unwrap();
        let branch_viewport: gtk::Viewport = self.builder.object("branch_viewport").unwrap();
        let branches_list: gtk::ListBox = self.builder.object("branches_list").unwrap();

        branch_window_grid.attach(&new_branch_label, 0, 0, 1, 1);

        entry_grid.add(&entry_for_new_branch);
        entry_grid.add(&apply_button);
        branch_window_grid.attach(&entry_grid, 0, 1, 1, 1);

        branch_window_grid.attach(&branch_names, 0, 2, 1, 1);

        branch_viewport.add(&branches_list);
        scrolled_window.add(&branch_viewport);
        branch_window_grid.attach(&scrolled_window, 0, 3, 1, 1);

        branch_window.add(&branch_window_grid);
        branch_window
    }

    fn set_right_area_ui(&mut self, commits: &Vec<(CommitObject, Option<String>)>) {
        let date_list: gtk::ListBox = self.builder.object("date_list").unwrap();
        let author_list: gtk::ListBox = self.builder.object("author_list").unwrap();
        let drawing_area: gtk::DrawingArea = DrawingArea::new();
        drawing_area.set_size_request(50, 50);
        let _stagin_changes_list: gtk::ListBox = self.builder.object("staging_list").unwrap();
        let description_list: gtk::ListBox = self.builder.object("description_list").unwrap();
        let commits_hashes_list: gtk::ListBox = self.builder.object("commit_hash_list").unwrap();
        let grid_drawing_area: gtk::Grid = self.builder.object("grid_drawing_area").unwrap();

        let children = grid_drawing_area.children();

        for child in &children {
            if let Some(drawing_area_old) = child.downcast_ref::<DrawingArea>() {
                grid_drawing_area.remove(drawing_area_old);
                break; // Termina el bucle después de eliminar el DrawingArea
            }
        }

        grid_drawing_area.attach(&drawing_area, 0, 1, 1, 1);
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);
        // drawing_area.queue_draw();

        remove_childs(&description_list);
        remove_childs(&date_list);
        remove_childs(&author_list);
        remove_childs(&commits_hashes_list);

        let mut hash_sons: HashMap<String, Vec<(f64, f64, String)>> = HashMap::new(); // hash, Vec<(x,y)> de los hijos
        let mut hash_branches: HashMap<String, usize> = HashMap::new();
        let mut identado: usize = 1;
        let mut y = 12;

        for commit_and_branches in commits {
            let mut commit = commit_and_branches.0.to_owned();
            add_row_to_list(&commit.get_message(), &description_list);
            add_row_to_list(&commit.get_timestamp_string(), &date_list);
            add_row_to_list(&commit.get_author().name, &author_list);
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

    fn initialize_merge(&mut self, path_file: String) {
        let label_path: gtk::Label = self.builder.object("merge_conflicts").unwrap();
        label_path.set_text(&format!("Merge Conflicts for: {}", path_file));

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
            line = String::new();
        }

        actualize_conflicts(
            "current",
            &mut new_content,
            &mut current_content,
            &mut incoming_content,
            &mut old_content,
            true,
        );

        let merge_window: gtk::Window = self.builder.object("merge_window").unwrap();
        merge_window.show_all();

        let buttons = ["current", "incoming", "next"];

        for button in buttons {
            self.merge_function_button(
                button,
                &new_content,
                &current_content,
                &incoming_content,
                &old_content,
            );
        }

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

        let clone_merge_window = merge_window.clone();
        clone_merge_window.connect_delete_event(move |_, _| {
            let merge_window = merge_window.clone();
            let mut builder = builder.clone();
            remove_widgets_to_merge_window(&mut builder, merge_window);
            Inhibit(false)
        });
    }

    fn merge_function_button(
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
                false,
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

fn remove_widgets_to_merge_window(builder: &mut gtk::Builder, merge_window: gtk::Window) {
    let merge_grid: gtk::Grid = builder.object("merge_grid").unwrap();
    let merge_conflicts_label: gtk::Label = builder.object("merge_conflicts").unwrap();
    let grid_buttons: gtk::Grid = builder.object("grid_buttons").unwrap();

    let accept_current_button: gtk::Button = builder.object("accept_current_button").unwrap();
    let accept_incoming_button: gtk::Button = builder.object("accept_incoming_button").unwrap();
    let accept_next_button: gtk::Button = builder.object("accept_next_button").unwrap();

    let current_scrolled: gtk::ScrolledWindow = builder.object("current_scrolled").unwrap();
    let viewport_current: gtk::Viewport = builder.object("viewport_current").unwrap();
    let current_label: gtk::Label = builder.object("current_content_label").unwrap();

    let old_scrolled: gtk::ScrolledWindow = builder.object("old_scrolled").unwrap();
    let viewport_old: gtk::Viewport = builder.object("viewport_old").unwrap();
    let old_label: gtk::Label = builder.object("old_content_label").unwrap();

    let incoming_scrolled: gtk::ScrolledWindow = builder.object("incoming_scrolled").unwrap();
    let viewport_incoming: gtk::Viewport = builder.object("viewport_incoming").unwrap();
    let incoming_label: gtk::Label = builder.object("incoming_content_label").unwrap();

    let new_scrolled: gtk::ScrolledWindow = builder.object("new_scrolled").unwrap();
    let viewport_new: gtk::Viewport = builder.object("viewport_new").unwrap();
    let new_label: gtk::Label = builder.object("new_content_label").unwrap();

    let finalize_conflict_button: gtk::Button = builder.object("finalize_conflict_button").unwrap();

    grid_buttons.remove(&accept_current_button);
    grid_buttons.remove(&accept_incoming_button);
    grid_buttons.remove(&accept_next_button);

    viewport_current.remove(&current_label);
    current_scrolled.remove(&viewport_current);

    viewport_old.remove(&old_label);
    old_scrolled.remove(&viewport_old);

    viewport_new.remove(&new_label);
    new_scrolled.remove(&viewport_new);

    viewport_incoming.remove(&incoming_label);
    incoming_scrolled.remove(&viewport_incoming);

    merge_grid.remove(&merge_conflicts_label);
    merge_grid.remove(&new_scrolled);
    merge_grid.remove(&grid_buttons);
    merge_grid.remove(&current_scrolled);
    merge_grid.remove(&incoming_scrolled);
    merge_grid.remove(&old_scrolled);
    merge_grid.remove(&finalize_conflict_button);

    merge_window.remove(&merge_grid);
}

fn remove_widgets_to_branch_window(builder: gtk::Builder, branch_window_clone: Window) {
    let builder = builder.clone();
    let branch_window_grid: gtk::Grid = builder.object("branch_window_grid").unwrap();
    let entry_grid: gtk::Grid = builder.object("entry_grid").unwrap();
    let scrolled_window: gtk::ScrolledWindow = builder.object("scrolled_window").unwrap();
    let new_branch_label: gtk::Label = builder.object("new_branch_label").unwrap();
    let branch_names: gtk::Label = builder.object("branch_names").unwrap();
    let entry_for_new_branch: gtk::Entry = builder.object("entry_for_new_branch").unwrap();
    let apply_button: gtk::Button = builder.object("apply_button").unwrap();
    let branch_viewport: gtk::Viewport = builder.object("branch_viewport").unwrap();
    let branches_list: gtk::ListBox = builder.object("branches_list").unwrap();

    branch_window_grid.remove(&new_branch_label);

    entry_grid.remove(&entry_for_new_branch);
    entry_grid.remove(&apply_button);

    branch_viewport.remove(&branches_list);
    scrolled_window.remove(&branch_viewport);

    branch_window_grid.remove(&scrolled_window);
    branch_window_grid.remove(&entry_grid);
    branch_window_grid.remove(&branch_names);
    branch_window_clone.remove(&branch_window_grid);
}

fn refresh_function(mut interface: Interface) -> ControlFlow<()> {
    // interface.staged_area_ui(); VER SI DESCOMENTAR
    let commits = match interface.actualizar() {
        Some(commits) => commits,
        None => return ControlFlow::Break(()),
    };
    let window = interface.principal_window.clone();
    interface.set_right_area_ui(&commits);
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
    let staging_area = repo.staging_area()?;
    let merge_conflicts = staging_area.get_unmerged_files();
    println!("MERGE CONFLICTS : {:?}", merge_conflicts);
    let mut files_merge_conflict = merge_conflicts.keys().cloned().collect::<HashSet<String>>();

    for conflict in &staging_changes {
        files_merge_conflict.remove(conflict);
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
    first_merge: bool,
) {
    let mut found_next_conflict = false;

    let mut current_content_lines: Vec<String> = Vec::new();
    let mut incoming_content_lines: Vec<String> = Vec::new();
    let mut old_new_content_lines: Vec<String> = Vec::new();

    let new_content_str = new_content.borrow_mut().clone();

    let current_content_str = current_content.borrow_mut().clone();
    let incoming_content_str = incoming_content.borrow_mut().clone();

    let mut new_content_lines = new_content_str.split('\n').collect::<Vec<&str>>();
    let current_old_content_lines = current_content_str.split('\n').collect::<Vec<&str>>();
    let incoming_old_content_lines = incoming_content_str.split('\n').collect::<Vec<&str>>();

    if accept_changes == "current" {
        new_content_lines.extend(current_old_content_lines);
    } else if accept_changes == "incoming" {
        new_content_lines.extend(incoming_old_content_lines);
    } else {
        new_content_lines.extend(current_old_content_lines);
        new_content_lines.extend(incoming_old_content_lines);
    }

    if first_merge {
        new_content_lines = Vec::new();
    }

    let mut old_new_content = String::new();

    let binding = old_content.borrow_mut().clone();
    let mut old_content_lines = binding.split('\n').collect::<Vec<&str>>();
    let mut i = 0;
    while let Some(line) = old_content_lines.get(i) {
        i += 1;
        if line.starts_with("<<<<<<<") && !found_next_conflict {
            found_next_conflict = true;
            while let Some(current_line) = old_content_lines.get_mut(i) {
                i += 1;
                if current_line.starts_with("=======") {
                    break;
                }
                current_content_lines.push(current_line.to_string());
            }

            while let Some(inner_line) = old_content_lines.get_mut(i) {
                i += 1;
                if inner_line.starts_with(">>>>>>>") {
                    break;
                }
                incoming_content_lines.push(inner_line.to_string());
            }
            continue;
        }

        if !found_next_conflict {
            new_content_lines.push(line.to_owned());
        } else {
            old_new_content_lines.push(line.to_string());
        }
    }

    new_content.borrow_mut().clear();
    current_content.borrow_mut().clear();
    incoming_content.borrow_mut().clear();
    old_content.borrow_mut().clear();

    old_new_content.push_str(&old_new_content_lines.join("\n"));
    new_content
        .borrow_mut()
        .push_str(&new_content_lines.join("\n"));
    incoming_content
        .borrow_mut()
        .push_str(&incoming_content_lines.join("\n"));
    current_content
        .borrow_mut()
        .push_str(&current_content_lines.join("\n"));

    old_content.borrow_mut().push_str(old_new_content.as_str());
}
