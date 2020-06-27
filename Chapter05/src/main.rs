extern crate gio;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate gtk_sys;
extern crate id3;

mod playlist;
mod toolbar;

use gio::{ApplicationExt, ApplicationFlags};
use std::rc::Rc;

use gtk::{
    Application,
    ApplicationWindow,
    ContainerExt,
    WidgetExt,
    WindowExt,
    Adjustment,
    Image,
    Scale,
    ScaleExt,
};
use gtk::Orientation::{Horizontal, Vertical};

use playlist::Playlist;
use toolbar::MusicToolbar;

fn main() {
    let application = Application::new("com.github.rust-by-example", ApplicationFlags::empty())
        .expect("Application initialization failed");
    application.connect_startup(|application| {
        let _app = App::new(application.clone());
    });
    application.connect_activate(|_| {});

    let original = ::std::env::args().collect::<Vec<_>>();
    let mut tmp = Vec::with_capacity(original.len());
    for i in 0..original.len() {
        tmp.push(original[i].as_str());
    }

    application.run(&tmp);
}

struct App {
    adjustment: Adjustment,
    cover: Image,
    playlist: Rc<Playlist>,
    toolbar: MusicToolbar,
    window: ApplicationWindow,
}

impl App {
    fn new(application: Application) -> Self {
        let window = ApplicationWindow::new(&application);
        window.set_title("Rusic");

        let vbox = gtk::Box::new(Vertical, 0);
        window.add(&vbox);

        let toolbar = MusicToolbar::new();
        vbox.add(toolbar.toolbar());

        let playlist = Rc::new(Playlist::new());
        vbox.add(playlist.view());

        let cover = Image::new();
        vbox.add(&cover);

        let adjustment = Adjustment::new(0.0, 0.0, 10.0, 0.0, 0.0, 0.0);
        let scale = Scale::new(Horizontal, &adjustment);
        scale.set_draw_value(false);
        vbox.add(&scale);

        window.show_all();

        let app = App {
            adjustment,
            cover,
            playlist,
            toolbar,
            window,
        };

        app.connect_events();
        app.connect_toolbar_events();

        app
    }

    fn connect_events(&self) {
    }
}
