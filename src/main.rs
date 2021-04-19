#[macro_use]
extern crate glib;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::Application;

use std::fs::File;
use std::io::prelude::*;

fn main() {
    let matches = clap::App::new("Device tree browser")
        .author("Windel Bouwman")
        .arg(clap::Arg::with_name("device_tree_file").help("Device tree file"))
        .get_matches();

    let optional_device_tree_filename: Option<String> =
        matches.value_of("device_tree_file").map(|s| s.to_owned());

    let application = Application::new(
        Some("com.github.windelbouwman.dtbrowse"),
        gio::ApplicationFlags::NON_UNIQUE,
    )
    .expect("failed to initialize GTK application");

    application.connect_activate(move |app| build_ui(app, &optional_device_tree_filename));

    application.run(&[]);
}

/// Load a device tree file into a GTK model.
fn make_model(filename: &str, model: &gtk::TreeStore) {
    let mut input = File::open(filename).unwrap();
    let mut buf = Vec::new();
    input.read_to_end(&mut buf).unwrap();

    let reader = dtb::Reader::read(buf.as_slice()).unwrap();

    let mut items = reader.struct_items();

    model.clear();

    let mut parents: Vec<Option<gtk::TreeIter>> = vec![None];
    while let Ok(item) = items.next_item() {
        match item {
            dtb::StructItem::BeginNode { name } => {
                let iter = model.append(parents.last().unwrap().clone().as_ref());
                model.set(&iter, &[0], &[&format!("{}", name)]);
                parents.push(Some(iter));
            }
            dtb::StructItem::Property { name, value } => {
                let iter = model.append(parents.last().unwrap().clone().as_ref());

                let mut buf2: Vec<u8> = value.to_vec();
                let value_txt: String = if let Ok(value) = item.value_str() {
                    value.to_owned()
                } else if let Ok(value) = item.value_u32_list(&mut buf2) {
                    format!("{:?}", value)
                } else {
                    format!("{:?}", value)
                };

                model.set(&iter, &[0, 1], &[&format!("{}", name), &value_txt]);
            }
            dtb::StructItem::EndNode => {
                parents.pop();
            }
        }
    }
}

fn build_ui(app: &gtk::Application, device_tree_filename: &Option<String>) {
    // Connect application to window:
    let window: gtk::Window = gtk::Window::new(gtk::WindowType::Toplevel);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 7);
    let button_expand_all = gtk::Button::new_with_label("Expand all!");
    let treeview = gtk::TreeView::new();
    let treeview_scrolled =
        gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
    treeview_scrolled.add(&treeview);

    // TODO: enable search filter:
    let search_box = gtk::SearchEntry::new();
    search_box.set_sensitive(false);

    // TODO: add logic to read file
    // let button_open_file = gtk::Button::new_with_label("Open file");
    vbox.pack_start(&button_expand_all, false, true, 0);
    vbox.pack_start(&search_box, false, true, 0);
    vbox.pack_start(&treeview_scrolled, true, true, 0);
    // vbox.pack_start(&button_open_file, false, true, 0);

    button_expand_all.connect_clicked(clone!(@strong treeview => move |_| {
        treeview.expand_all();
    }));

    let model = gtk::TreeStore::new(&[String::static_type(), String::static_type()]);

    // If we passed some device tree file, load it now:
    if let Some(device_tree_filename) = device_tree_filename {
        make_model(&device_tree_filename, &model);
    }

    let filter_model = gtk::TreeModelFilter::new(&model, None);

    filter_model.set_visible_func(clone!(@strong search_box => move |m, i| {
        let txt = search_box.get_text().unwrap().to_string();
        signal_filter_func(m, i, txt)
    }));

    treeview.set_model(Some(&filter_model));

    search_box.connect_search_changed(move |_e| {
        filter_model.refilter();
    });

    let name_column = gtk::TreeViewColumn::new();
    name_column.set_title("name");
    let value_column = gtk::TreeViewColumn::new();
    value_column.set_title("value");

    // treeview columns
    let cell = gtk::CellRendererText::new();
    name_column.pack_start(&cell, true);
    name_column.add_attribute(&cell, "text", 0);

    let cell = gtk::CellRendererText::new();
    value_column.pack_start(&cell, true);
    value_column.add_attribute(&cell, "text", 1);

    treeview.append_column(&name_column);
    treeview.append_column(&value_column);

    treeview.expand_all();

    window.add(&vbox);
    window.set_application(Some(app));
    window.show_all();
}

fn signal_filter_func(model: &gtk::TreeModel, iter: &gtk::TreeIter, filter_txt: String) -> bool {
    let optional_name = model.get_value(&iter, 0).get::<String>().unwrap();
    if let Some(name) = optional_name {
        filter_txt.is_empty() || name.contains(&filter_txt)
    } else {
        true
    }
}
