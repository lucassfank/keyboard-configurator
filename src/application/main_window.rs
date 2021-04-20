use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use super::{shortcuts_window, ConfiguratorApp, Keyboard, KeyboardLayer, Page, Picker};
use crate::DerefCell;
use backend::{Backend, Board};

pub struct Loader(MainWindow, gtk::Box);

impl Drop for Loader {
    fn drop(&mut self) {
        self.0.inner().load_box.remove(&self.1);
        let mut empty = true;
        self.0.inner().load_box.foreach(|_| empty = true);
        if empty {
            self.0.inner().load_revealer.set_reveal_child(false);
        }
    }
}

#[derive(Default)]
pub struct MainWindowInner {
    backend: DerefCell<Backend>,
    back_button: DerefCell<gtk::Button>,
    count: AtomicUsize,
    header_bar: DerefCell<gtk::HeaderBar>,
    keyboard_list_box: DerefCell<gtk::ListBox>,
    layer_switcher: DerefCell<gtk::StackSwitcher>,
    load_box: DerefCell<gtk::Box>,
    load_revealer: DerefCell<gtk::Revealer>,
    picker: DerefCell<Picker>,
    stack: DerefCell<gtk::Stack>,
    keyboards: RefCell<Vec<(Keyboard, gtk::ListBoxRow)>>,
    board_loading: RefCell<Option<Loader>>,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWindowInner {
    const NAME: &'static str = "S76ConfiguratorMainWindow";
    type ParentType = gtk::ApplicationWindow;
    type Type = MainWindow;
}

impl ObjectImpl for MainWindowInner {
    fn constructed(&self, window: &MainWindow) {
        self.parent_constructed(window);

        let back_button = cascade! {
            gtk::Button::new();
            ..add(&gtk::Image::from_icon_name(Some("go-previous-symbolic"), gtk::IconSize::Button));
            ..connect_clicked(clone!(@weak window => move |_| {
                window.show_keyboard_list();
            }));
        };

        let layer_switcher = gtk::StackSwitcher::new();

        let menu = cascade! {
            gio::Menu::new();
            ..append_section(None, &cascade! {
                gio::Menu::new();
                ..append(Some("Load Layout"), Some("kbd.load"));
                ..append(Some("Save Layout"), Some("kbd.save"));
                ..append(Some("Reset Layout"), Some("kbd.reset"));
            });
            ..append_section(None, &cascade! {
                gio::Menu::new();
                ..append(Some("Keyboard Shortcuts"), Some("win.show-help-overlay"));
                ..append(Some("About Keyboard Configurator"), Some("app.about"));
            });
        };

        let header_bar = cascade! {
            gtk::HeaderBar::new();
            ..set_show_close_button(true);
            ..pack_start(&back_button);
            ..set_custom_title(Some(&layer_switcher));
            ..pack_end(&cascade! {
                gtk::MenuButton::new();
                ..set_menu_model(Some(&menu));
                ..add(&cascade! {
                    gtk::Image::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Button);
                });
            });
        };

        let no_boards_msg = concat! {
            "<span size='x-large' weight='bold'>No keyboard detected</span>\n",
            "Make sure your built-in keyboard has up to date\n",
            "System76 Open Firmware.\n",
            "If using an external keyboard, make sure it is\n",
            "plugged in properly.",
        };
        let no_boards = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 24);
            ..add(&cascade! {
                gtk::Image::from_icon_name(Some("input-keyboard-symbolic"), gtk::IconSize::Invalid);
                ..set_pixel_size(256);
                ..set_halign(gtk::Align::Center);
            });
            ..add(&cascade! {
                gtk::Label::new(Some(no_boards_msg));
                ..set_justify(gtk::Justification::Center);
                ..set_use_markup(true);
            });
            ..show_all();
        };

        let keyboard_list_box = cascade! {
            gtk::ListBox::new();
            ..set_placeholder(Some(&no_boards));
            ..set_halign(gtk::Align::Center);
            ..get_style_context().add_class("frame");
            ..set_property_margin(6);
        };

        let stack = cascade! {
            gtk::Stack::new();
            ..set_homogeneous(false);
            ..add(&keyboard_list_box);
        };

        let picker = Picker::new();

        let load_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 6);
            ..set_property_margin(6);
            ..show();
        };

        let load_revealer = cascade! {
            gtk::Revealer::new();
            ..set_transition_type(gtk::RevealerTransitionType::SlideDown);
            ..add(&load_box);
        };

        cascade! {
            window;
            ..set_title("System76 Keyboard Configurator");
            ..set_position(gtk::WindowPosition::Center);
            ..set_default_size(1024, 768);
            ..set_titlebar(Some(&header_bar));
            ..add(&cascade! {
                gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
                ..add(&cascade! {
                    gtk::Box::new(gtk::Orientation::Vertical, 0);
                    ..add(&load_revealer);
                    ..add(&stack);
                });
            });
            ..set_help_overlay(Some(&shortcuts_window()));
            ..set_focus(None::<&gtk::Widget>);
            ..show_all();
        };
        back_button.set_visible(false);

        self.back_button.set(back_button);
        self.header_bar.set(header_bar);
        self.keyboard_list_box.set(keyboard_list_box);
        self.layer_switcher.set(layer_switcher);
        self.load_box.set(load_box);
        self.load_revealer.set(load_revealer);
        self.picker.set(picker);
        self.stack.set(stack);
    }
}
impl WidgetImpl for MainWindowInner {
    fn destroy(&self, window: &MainWindow) {
        self.parent_destroy(window);
        info!("Window close");
    }
}
impl ContainerImpl for MainWindowInner {}
impl BinImpl for MainWindowInner {}
impl WindowImpl for MainWindowInner {}
impl ApplicationWindowImpl for MainWindowInner {}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<MainWindowInner>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Bin, gtk::Container, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MainWindow {
    pub fn new(app: &ConfiguratorApp) -> Self {
        let window: Self = glib::Object::new(&[]).unwrap();
        app.add_window(&window);

        let backend = cascade! {
            daemon();
            ..connect_board_loading(clone!(@weak window => move || {
                let loader = window.display_loader("Keyboard(s) detected. Loading...");
                *window.inner().board_loading.borrow_mut() = Some(loader);
            }));
            ..connect_board_loading_done(clone!(@weak window => move || {
                window.inner().board_loading.borrow_mut().take();
            }));
            ..connect_board_added(clone!(@weak window => move |board| window.add_keyboard(board)));
            ..connect_board_removed(clone!(@weak window => move |board| {
                let mut boards = window.inner().keyboards.borrow_mut();
                if let Some(idx) = boards.iter().position(|(kb, _)| kb.board() == &board) {
                    let (keyboard, row) = boards.remove(idx);
                    window.inner().stack.remove(&keyboard);
                    window.inner().keyboard_list_box.remove(&row);
                }
            }));
            ..refresh();
        };

        // Refresh key matrix only when window is visible
        backend.set_matrix_get_rate(if window.is_active() {
            Some(Duration::from_millis(50))
        } else {
            None
        });
        window.connect_property_is_active_notify(clone!(@weak backend => move |window| {
            backend.set_matrix_get_rate(if window.is_active() {
                Some(Duration::from_millis(50))
            } else {
                None
            });
        }));

        let phony_board_names = app.phony_board_names().to_vec();
        if !phony_board_names.is_empty() {
            let backend = Backend::new_dummy(phony_board_names).unwrap();
            backend.connect_board_added(
                clone!(@weak window => move |board| window.add_keyboard(board)),
            );
            backend.refresh();
        }

        window.inner().backend.set(backend);
        glib::timeout_add_seconds_local(
            1,
            clone!(@weak window => @default-return glib::Continue(false), move || {
                window.inner().backend.refresh();
                glib::Continue(true)
            }),
        );

        window
    }

    fn inner(&self) -> &MainWindowInner {
        MainWindowInner::from_instance(self)
    }

    fn show_keyboard_list(&self) {
        let inner = self.inner();
        inner
            .stack
            .set_transition_type(gtk::StackTransitionType::SlideRight);
        inner.stack.set_visible_child(&*inner.keyboard_list_box);
        inner.header_bar.set_custom_title::<gtk::Widget>(None);
        inner.back_button.set_visible(false);
    }

    fn show_keyboard(&self, keyboard: &Keyboard) {
        let inner = self.inner();

        inner
            .stack
            .set_transition_type(gtk::StackTransitionType::SlideLeft);
        inner.stack.set_visible_child(keyboard);
        inner
            .header_bar
            .set_custom_title(Some(&*inner.layer_switcher));
        inner.layer_switcher.set_stack(Some(keyboard.layer_stack()));
        self.insert_action_group("kbd", Some(keyboard.action_group()));
        inner.back_button.set_visible(true);

        inner.picker.set_keyboard(Some(keyboard.clone()));
    }

    fn add_keyboard(&self, board: Board) {
        let app: ConfiguratorApp = self.get_application().unwrap().downcast().unwrap();

        let keyboard = cascade! {
            Keyboard::new(board, app.debug_layers());
            ..set_halign(gtk::Align::Center);
            ..show_all();
        };

        let attr_list = cascade! {
            pango::AttrList::new();
            ..insert(pango::Attribute::new_weight(pango::Weight::Bold));
        };
        let label = cascade! {
            gtk::Label::new(Some(&keyboard.display_name()));
            ..set_attributes(Some(&attr_list));
        };
        let window = self;
        let button = cascade! {
            gtk::Button::with_label("Configure Layout");
            ..set_halign(gtk::Align::Center);
            ..connect_clicked(clone!(@weak window, @weak keyboard => move |_| {
                window.show_keyboard(&keyboard);
            }));
        };
        let keyboard_layer = cascade! {
            KeyboardLayer::new(Page::Keycaps, keyboard.board().clone());
            ..set_halign(gtk::Align::Center);
        };
        let keyboard_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&label);
            ..add(&keyboard_layer);
            ..add(&button);
        };
        let row = cascade! {
            gtk::ListBoxRow::new();
            ..set_activatable(false);
            ..set_selectable(false);
            ..add(&keyboard_box);
            ..set_margin_top(12);
            ..set_margin_bottom(12);
            ..show_all();
        };
        self.inner().keyboard_list_box.add(&row);

        self.inner().stack.add(&keyboard);
        self.inner().keyboards.borrow_mut().push((keyboard, row));

        // XXX if only one keyboard, show that with no back button
        self.inner().count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn display_loader(&self, text: &str) -> Loader {
        let load_hbox = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 6);
            ..add(&cascade! {
                gtk::Spinner::new();
                ..start();
            });
            ..add(&gtk::Label::new(Some(text)));
            ..show_all();
        };

        self.inner().load_box.add(&load_hbox);
        self.inner().load_revealer.set_reveal_child(true);

        Loader(self.clone(), load_hbox)
    }
}

#[cfg(target_os = "linux")]
fn daemon() -> Backend {
    if unsafe { libc::geteuid() == 0 } {
        info!("Already running as root");
        Backend::new()
    } else {
        info!("Not running as root, spawning daemon with pkexec");
        Backend::new_pkexec()
    }
    .expect("Failed to create server")
}

#[cfg(not(target_os = "linux"))]
fn daemon() -> Backend {
    Backend::new().expect("Failed to create server")
}
