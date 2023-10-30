extern crate gtk;
use std::collections::HashMap;

use git::*;
use git_lib::objects::{author, commit_object::CommitObject};
// use git_lib::*;
use gtk::{prelude::*, DrawingArea, Label, ListBox, ListBoxRow};

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

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let commits = git::commands::log::Log::run_for_graph().unwrap();

    let glade_src = include_str!("../../git_interface.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let window: gtk::Window = builder.object("window app").unwrap();

    set_buttons();

    let stagin_changes_list: gtk::ListBox = builder.object("lista_staging_changes").unwrap();

    let grafo_list: gtk::ListBox = builder.object("grafo_list").unwrap();
    let description_list: gtk::ListBox = builder.object("description_list").unwrap();
    let date_list: gtk::ListBox = builder.object("date_list").unwrap();
    let author_list: gtk::ListBox = builder.object("author_list").unwrap();
    let commits_hashes_list: gtk::ListBox = builder.object("commit_hash_list").unwrap();

    set_graph(
        grafo_list,
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

fn set_graph(
    grafo_list: ListBox,
    description_list: ListBox,
    date_list: ListBox,
    author_list: ListBox,
    commits_hashes_list: ListBox,
    commits: Vec<(CommitObject, Option<String>)>,
) {
    let mut hash_branches: HashMap<String, usize> = HashMap::new();
    let mut identado: usize = 1;
    for commit_and_branches in commits {
        identado = make_graph(
            &grafo_list,
            &mut hash_branches,
            &mut identado,
            &commit_and_branches,
        );

        let mut commit = commit_and_branches.0;
        add_row_to_list(&commit.message, &description_list);
        add_row_to_list(&commit.timestamp.to_string(), &date_list);
        add_row_to_list(&commit.author.to_string(), &author_list);
        add_row_to_list(&commit.get_hash_string().unwrap(), &commits_hashes_list);
    }
}

fn make_graph(
    grafo_list: &ListBox,
    hash_branches: &mut HashMap<String, usize>,
    identado: &mut usize,
    commit: &(CommitObject, Option<String>),
) -> usize {
    let drawing_area = DrawingArea::new();
    drawing_area.set_size_request(20, 20);

    draw_lines_branches(hash_branches, &drawing_area);
    let commit_branch = commit.1.as_ref().unwrap();
    if !hash_branches.contains_key(commit_branch) {
        let index_color = *identado % GRAPH_COLORS.len();
        let color = GRAPH_COLORS[index_color];
        hash_branches.insert(commit_branch.clone(), *identado);
        *identado += 1;
    }

    // drawing_area.connect_draw(|_, context| {
    //     // Dibuja una línea en el DrawingArea
    //     context.set_source_rgb(1.0, 1.0, 0.0);
    //     context.set_line_width(5.0);
    //     context.move_to(10.0, 10.0);
    //     context.line_to(190.0, 190.0);
    //     context.stroke();
    //     Inhibit(false)
    // });

    let row_graph = ListBoxRow::new();
    row_graph.add(&drawing_area);
    grafo_list.add(&row_graph);

    return *identado;
}

fn draw_lines_branches(hash_branches: &mut HashMap<String, usize>, drawing_area: &DrawingArea) {
    let color = GRAPH_COLORS[0];
    // definimos el tamaño del DrawingArea
    drawing_area.set_size_request(50, 50);

    drawing_area.connect_draw(move |_, context| {
        // Dibuja una línea en el DrawingArea
        context.set_source_rgb(color.0, color.1, color.2);
        context.set_line_width(1.0);
        context.move_to(20.0, 0.0);
        context.line_to(20.0, 190.0);
        context.stroke();
        Inhibit(false)
    });
}

fn add_row_to_list(row_information: &String, row_list: &ListBox) {
    let label = Label::new(Some(&row_information));
    let row_date = ListBoxRow::new();
    row_date.add(&label);
    row_list.add(&row_date);
}

// fn add_

// for _ in 1..50 {
//     let drawing_area = DrawingArea::new();
//     drawing_area.set_size_request(300, 300);
//     drawing_area.connect_draw(|_, context| {
//         // Dibuja una línea en el DrawingArea
//         context.set_source_rgb(1.0, 1.0, 0.0);
//         context.set_line_width(5.0);
//         context.move_to(10.0, 10.0);
//         context.line_to(190.0, 190.0);
//         context.stroke();
//         Inhibit(false)
//     });
//     stagin_changes_list.add(&drawing_area);
// }
// }

fn set_buttons() {
    // let commit: gtk::Button = builder.object("commit").unwrap();
    // let more_options: gtk::Button = builder.object("more options").unwrap();
    // let git_graph: gtk::Button = builder.object("git graph").unwrap();
    // let refresh: gtk::Button = builder.object("refresh").unwrap();
    // let mensaje_commit: gtk::Entry = builder.object("mensaje commit").unwrap();
}

// commit.connect_clicked(move |_| {
//     if mensaje_commit.text().len() == 0 {
//         let dialog = gtk::MessageDialog::new(
//             Some(&window),
//             gtk::DialogFlags::MODAL,
//             gtk::MessageType::Error,
//             gtk::ButtonsType::Ok,
//             "No se ha ingresado un mensaje de commit",
//         );
//         dialog.run();
//         dialog.hide();
//     } else {
//         let dialog = gtk::MessageDialog::new(
//             Some(&window),
//             gtk::DialogFlags::MODAL,
//             gtk::MessageType::Info,
//             gtk::ButtonsType::Ok,
//             "Commit realizado con exito",
//         );
//         dialog.run();
//         dialog.hide();
//     }
// });
