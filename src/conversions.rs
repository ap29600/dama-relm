use crate::helper::*;
use crate::structs::*;
use crate::watch::*;
use gtk::prelude::*;
use notify::{RecursiveMode, Watcher};
use std::path::Path;

#[macro_export]
macro_rules! add_css{
    ($css:expr, $($widget:expr),*) => (
        {
            $(
                if let Some(css) = $css {
                    let provider = gtk::CssProvider::new();
                    match provider.load_from_data(css.as_bytes()) {
                        Ok(_) => $widget
                            .get_style_context()
                            .add_provider(&provider,
                                          gtk::STYLE_PROVIDER_PRIORITY_USER),
                        Err(e) => eprint!("CSS: {}", e),
                    }
                }
            )*
        }
    );
}

#[macro_export]
macro_rules! add_name{
    ($name:expr, $($widget:expr),*) => (
        {
            $(
                if let Some(name) = $name {
                    $widget.set_widget_name(&name);
                }
            )*
        }
    );
}

impl From<ComboBox> for gtk::ComboBoxText {
    fn from(bx: ComboBox) -> Self {
        let ComboBox {
            initialize,
            select,
            on_update,
            css,
            name,
            watch,
        } = bx;

        let combo = gtk::ComboBoxText::new();
        read_stdout_from_command(&initialize)
            .split('\n')
            .filter(|&line| !line.is_empty())
            .for_each(|entry| combo.append(Some(entry), entry));

        let active_value = read_value_from_command::<String>(&select);
        combo.set_active_id(active_value.as_deref());

        if let Some(watch) = watch {
            let moved_select = select.clone();

            let (tx, rx) = glib::MainContext::channel(glib::Priority::default());
            std::thread::spawn(move || {
                let mut watcher =
                    notify::recommended_watcher(move |res: Result<notify::Event, _>| match res {
                        Ok(event) => {
                            if let notify::EventKind::Modify(_) = event.kind {
                                let active_value = read_value_from_command::<String>(&moved_select);
                                tx.send(active_value).ok().unwrap();
                            }
                        }
                        Err(error) => println!("[ERROR] {:?}", error),
                    })
                    .unwrap();
                watcher
                    .watch(Path::new(&watch), RecursiveMode::Recursive)
                    .ok();
                // the thread needs to remain alive, otherwise the watcher will be dropped..
                loop {
                    std::thread::yield_now();
                }
            });

            let combo_clone = combo.clone();
            rx.attach(None, move |msg| {
                // TODO: check if the scale is being dragged, and in that case
                // avoid changing the value.
                if !combo_clone
                    .get_state_flags()
                    .contains(gtk::StateFlags::ACTIVE | gtk::StateFlags::PRELIGHT)
                {
                    combo_clone.set_active_id(msg.as_deref());
                }
                glib::Continue(true)
            });
        }

        combo.connect_changed(move |combo| {
            std::env::set_var("DAMA_VAL", combo.get_active_text().unwrap());
            // if the command was not successful, we run the init script again
            if !execute_shell_command(&on_update) {
                let now_active = read_value_from_command::<String>(&select);
                combo.set_active_id(now_active.as_deref());
            }
        });
        add_name!(name, combo);
        add_css!(css, combo);
        combo
    }
}

impl From<Scale> for gtk::Scale {
    fn from(sc: Scale) -> Self {
        let Scale {
            range,
            initialize,
            on_update,
            css,
            name,
            watch,
        } = sc;

        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, range.low, range.high, 5.);
        let initial_value = read_value_from_command(&initialize).unwrap_or(range.low);
        scale.set_size_request(250, 12);
        scale.set_value(initial_value);

        if let Some(watch) = watch {
            let (tx, rx) = glib::MainContext::channel(glib::Priority::default());
            std::thread::spawn(move || {
                let mut watcher =
                    notify::recommended_watcher(move |res: Result<notify::Event, _>| match res {
                        Ok(event) => {
                            if let notify::EventKind::Modify(_) = event.kind {
                                let new_val =
                                    read_value_from_command(&initialize).unwrap_or(range.low);
                                tx.send(new_val).ok().unwrap();
                            }
                        }
                        Err(error) => println!("[ERROR] {:?}", error),
                    })
                    .unwrap();
                watcher
                    .watch(Path::new(&watch), RecursiveMode::Recursive)
                    .ok();
                // the thread needs to remain alive, otherwise the watcher will be dropped..
                loop {
                    std::thread::yield_now();
                }
            });

            let scale_clone = scale.clone();
            rx.attach(None, move |msg| {
                // TODO: check if the scale is being dragged, and in that case
                // avoid changing the value.
                if !scale_clone
                    .get_state_flags()
                    .contains(gtk::StateFlags::ACTIVE | gtk::StateFlags::PRELIGHT)
                {
                    scale_clone.set_value(msg);
                }
                glib::Continue(true)
            });
        }

        let tx = Watch::new(initial_value);
        let mut rx = tx.clone();
        std::thread::spawn(move || loop {
            std::env::set_var("DAMA_VAL", rx.wait().floor().to_string());
            execute_shell_command(&on_update);
        });
        scale.connect_change_value(move |_, _, new_value| {
            tx.clone().set_value(new_value);
            Inhibit(false)
        });
        add_name!(name, scale);
        add_css!(css, scale);
        scale
    }
}

impl From<Image> for gtk::Image {
    fn from(im: Image) -> Self {
        let Image { path, css, name } = im;

        let image = gtk::Image::from_file(path);
        image.set_margin_top(10);
        image.set_margin_bottom(10);
        image.set_margin_start(10);
        image.set_margin_end(10);
        add_name!(name, image);
        add_css!(css, image);
        image
    }
}

impl From<Label> for gtk::Label {
    fn from(lb: Label) -> Self {
        let Label { text, css, name } = lb;

        let label = gtk::Label::new(None);
        label.set_markup(&text);
        label.set_line_wrap(true);
        label.set_xalign(0.0);
        add_name!(name, label);
        add_css!(css, label);
        label
    }
}

impl From<CheckBox> for gtk::CheckButton {
    fn from(cb: CheckBox) -> Self {
        let CheckBox {
            text,
            initialize,
            on_click,
            css,
            name,
            watch,
        } = cb;

        let checkbox = gtk::CheckButton::with_label(&text);
        let initial_value = read_value_from_command(&initialize).unwrap_or(false);

        if let Some(watch) = watch {
            let (tx, rx) = glib::MainContext::channel(glib::Priority::default());

            std::thread::spawn(move || {
                let mut watcher =
                    notify::recommended_watcher(move |res: Result<notify::Event, _>| match res {
                        Ok(event) => {
                            if let notify::EventKind::Modify(_) = event.kind {
                                let new_val = read_value_from_command(&initialize).unwrap_or(false);
                                tx.send(new_val).ok().unwrap();
                            }
                        }
                        Err(error) => println!("[ERROR] {:?}", error),
                    })
                    .unwrap();

                watcher
                    .watch(Path::new(&watch), RecursiveMode::Recursive)
                    .ok();

                // the thread needs to remain alive, otherwise the watcher will be dropped..
                loop {
                    std::thread::yield_now();
                }
            });

            let checkbox_clone = checkbox.clone();
            rx.attach(None, move |msg| {
                if checkbox_clone.get_active() != msg {
                    checkbox_clone.set_active(msg);
                }
                glib::Continue(true)
            });
        }

        checkbox.set_active(initial_value);
        checkbox.connect_toggled(move |checkbox| {
            std::env::set_var("DAMA_VAL", checkbox.get_active().to_string());
            execute_shell_command(&on_click);
        });

        add_name!(name, checkbox);
        add_css!(css, checkbox);
        checkbox
    }
}

impl From<Button> for gtk::Button {
    fn from(bt: Button) -> Self {
        let Button {
            text,
            on_click,
            css,
            name,
        } = bt;

        let button = gtk::Button::with_label(&text);
        button.connect_clicked(move |_| {
            execute_shell_command(&on_click);
        });
        add_name!(name, button);
        add_css!(css, button);
        button
    }
}

use crate::ui_builder::AddFromSerializable;
impl From<Notebook> for gtk::Notebook {
    fn from(nb: Notebook) -> Self {
        let Notebook {
            children,
            css,
            name,
        } = nb;
        let notebook = gtk::Notebook::new();
        notebook.set_tab_pos(gtk::PositionType::Left);
        add_name!(name, notebook);
        add_css!(css, notebook);
        for child in children {
            notebook.add_from(child);
        }
        notebook
    }
}

impl From<Box> for gtk::Box {
    fn from(b: Box) -> Self {
        let Box {
            title: _,
            orientation,
            children,
            css,
            name,
        } = b;
        let gtkbox = gtk::Box::new(
            orientation.into(),
            match orientation {
                OrientationSerial::Horizontal => 30,
                _ => 0,
            },
        );
        gtkbox.set_border_width(10);
        // would be nice to just stop listening to draw signals after the first one
        // but gtk does not expose a connect_first_draw() function or similar;
        // there is probably a better way to do this.
        gtkbox.connect_draw(move |gtkbox, _| {
            // only populate the box when drawing, if empty.
            // this way if you have many pages running
            // intensive scripts only the ones you actually use
            // will be loaded.
            if gtkbox.get_children().is_empty() {
                for child in children.clone() {
                    gtkbox.add_from(child);
                }
                // if the first element is a label make
                // it expand to push other stuff aside
                if let Some(w) = gtkbox.get_children().get(0) {
                    if w.is::<gtk::Label>() {
                        gtkbox.set_child_packing(
                            w,
                            true, // expand
                            true, // fill
                            12,   // padding
                            gtk::PackType::Start,
                        );
                    }
                }
                gtkbox.show_all();
            }
            Inhibit(false)
        });
        add_name!(name, gtkbox);
        add_css!(css, gtkbox);
        gtkbox
    }
}
