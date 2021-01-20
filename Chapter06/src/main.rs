/*
 * FIXME: sometimes, after stopping, the next play does not work.
 *
 * TODO: change the volume.
 */

extern crate crossbeam;
extern crate gdk_pixbuf;
extern crate gio;
extern crate gtk;
extern crate gtk_sys;
extern crate id3;
extern crate m3u;
extern crate pulse_simple;
extern crate simplemad;

mod mp3;
mod player;
mod playlist;
mod toolbar;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gio::{ApplicationExt, ApplicationFlags};
use gtk::{
    Adjustment,
    AdjustmentExt,
    Application,
    ApplicationWindow,
    ContainerExt,
    Continue,
    Image,
    Inhibit,
    Label,
    LabelExt,
    Scale,
    ScaleExt,
    WidgetExt,
    WindowExt,
};
use gtk::Orientation::{Horizontal, Vertical};

use playlist::Playlist;
use toolbar::{MusicToolbar, set_image_icon, PAUSE_ICON, PLAY_ICON};

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

struct State {
    current_time: u64,
    durations: HashMap<String, u64>,
    stopped: bool,
}

struct App {
    adjustment: Adjustment,
    application: Application,
    cover: Image,
    current_time_label: Label,
    duration_label: Label,
    playlist: Rc<Playlist>,
    state: Arc<Mutex<State>>,
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

        let current_time = 0;
        let durations = HashMap::new();
        let state = Arc::new(Mutex::new(State {
            current_time,
            durations,
            stopped: true,
        }));

        let playlist = Rc::new(Playlist::new(state.clone()));
        vbox.add(playlist.view());

        let cover = Image::new();
        vbox.add(&cover);

        let hbox = gtk::Box::new(Horizontal, 10);
        vbox.add(&hbox);

        let adjustment = Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let scale = Scale::new(Horizontal, &adjustment);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        hbox.add(&scale);

        let current_time_label = Label::new(None);
        hbox.add(&current_time_label);

        let slash_label = Label::new("/");
        hbox.add(&slash_label);

        let duration_label = Label::new(None);
        duration_label.set_margin_right(10);
        hbox.add(&duration_label);

        window.show_all();

        let app = App {
            adjustment,
            application,
            cover,
            current_time_label,
            duration_label,
            playlist,
            state,
            toolbar,
            window,
        };

        app.connect_events();
        app.connect_toolbar_events();

        app
    }

    fn connect_events(&self) {
        let application = self.application.clone();
        self.window.connect_delete_event(move |_, _| {
            application.quit();
            Inhibit(false)
        });

        let current_time_label = self.current_time_label.clone();
        let duration_label = self.duration_label.clone();
        let playlist = self.playlist.clone();
        let adjustment = self.adjustment.clone();
        let state = self.state.clone();
        let play_image = self.toolbar.play_image.clone();
        gtk::timeout_add(100, move || {
            let state = state.lock().unwrap();
            if let Some(path) = playlist.path() {
                if let Some(&duration) = state.durations.get(&path) {
                    adjustment.set_upper(duration as f64);
                    duration_label.set_text(&millis_to_minutes(duration));
                }
            }
            if state.stopped {
                set_image_icon(&play_image, PLAY_ICON);
            } else {
                set_image_icon(&play_image, PAUSE_ICON);
                current_time_label.set_text(&millis_to_minutes(state.current_time));
            }
            adjustment.set_value(state.current_time as f64);
            Continue(true)
        });
    }
}

fn to_millis(duration: Duration) -> u64 {
    duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1_000_000
}

fn millis_to_minutes(millis: u64) -> String {
    let mut seconds = millis / 1_000;
    let minutes = seconds / 60;
    seconds %= 60;
    format!("{}:{:02}", minutes, seconds)
}
