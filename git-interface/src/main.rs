extern crate gtk;
use gtk::{prelude::*, Adjustment, DrawingArea, ListBox, ScrolledWindow, Window, WindowType};

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    

    let glade_src = include_str!("../../git_interface.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::Window = builder.object("window app").unwrap();

    // let commit: gtk::Button = builder.object("commit").unwrap();
    // let more_options: gtk::Button = builder.object("more options").unwrap();
    // let git_graph: gtk::Button = builder.object("git graph").unwrap();
    // let refresh: gtk::Button = builder.object("refresh").unwrap();
    // let mensaje_commit: gtk::Entry = builder.object("mensaje commit").unwrap();

    let stagin_changes_list: gtk::ListBox = builder.object("lista_staging_changes").unwrap();

    for _ in 1..50 {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(300, 300);
        drawing_area.connect_draw(|_, context| {
            // Dibuja una línea en el DrawingArea
            context.set_source_rgb(1.0, 1.0, 0.0);
            context.set_line_width(5.0);
            context.move_to(10.0, 10.0);
            context.line_to(190.0, 190.0);
            context.stroke();
            Inhibit(false)
        });
        stagin_changes_list.add(&drawing_area);
    }

    // // Agregar la ListBox al ScrolledWindow
    // scrolled_window.add(&list_box);

    // // Agregar el ScrolledWindow a la ventana principal
    // window.add(&scrolled_window);

    // Conectar la señal "delete_event" para cerrar la ventana
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.show_all();

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

    gtk::main();
}
