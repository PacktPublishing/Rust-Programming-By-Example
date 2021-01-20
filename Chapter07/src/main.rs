#![feature(proc_macro)]

extern crate crossbeam;
extern crate futures;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate gtk_sys;
extern crate id3;
extern crate m3u;
extern crate pulse_simple;
#[macro_use]
extern crate relm;
extern crate relm_attributes;
#[macro_use]
extern crate relm_derive;
extern crate simplemad;

mod mp3;
mod player;
mod playlist;

use std::path::PathBuf;
use std::time::Duration;

use gdk_pixbuf::Pixbuf;
use gtk::{
    Adjustment,
    AdjustmentExt,
    BoxExt,
    ButtonsType,
    DialogExt,
    DialogFlags,
    FileChooserAction,
    FileChooserDialog,
    FileChooserExt,
    FileFilter,
    FileFilterExt,
    GtkWindowExt,
    Image,
    ImageExt,
    Inhibit,
    LabelExt,
    MessageDialog,
    MessageType,
    OrientableExt,
    ScaleExt,
    ToolButtonExt,
    WidgetExt,
    Window,
};
use gtk::Orientation::{Horizontal, Vertical};
use gtk_sys::{GTK_RESPONSE_ACCEPT, GTK_RESPONSE_CANCEL};
use relm::Widget;
use relm_attributes::widget;

use playlist::{PlayerMsg, Playlist};
use playlist::PlayerMsg::{PlayerPlay, PlayerStop, PlayerTime};
use playlist::Msg::{
    AddSong,
    LoadSong,
    NextSong,
    PlayerMsgRecv,
    PlaySong,
    PauseSong,
    PreviousSong,
    RemoveSong,
    SaveSong,
    SongDuration,
    SongStarted,
    StopSong,
};
use self::Msg::*;

pub const PAUSE_ICON: &str = "gtk-media-pause";
pub const PLAY_ICON: &str = "gtk-media-play";
const RESPONSE_ACCEPT: i32 = GTK_RESPONSE_ACCEPT as i32;
const RESPONSE_CANCEL: i32 = GTK_RESPONSE_CANCEL as i32;

fn main() {
    App::run(()).unwrap();
}

pub struct Model {
    adjustment: Adjustment,
    cover_pixbuf: Option<Pixbuf>,
    cover_visible: bool,
    current_duration: u64,
    current_time: u64,
    play_image: Image,
    stopped: bool,
}

#[derive(Msg)]
pub enum Msg {
    Duration(u64),
    MsgRecv(PlayerMsg),
    Next,
    Open,
    PlayPause,
    Previous,
    Quit,
    Remove,
    Save,
    Started(Option<Pixbuf>),
    Stop,
}

#[widget]
impl Widget for App {
    fn init_view(&mut self) {
        // NOTE: needed because relm only calls show(), not show_all().
        self.toolbar.show_all();
    }

    fn model() -> Model {
        Model {
            adjustment: Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            cover_pixbuf: None,
            cover_visible: false,
            current_duration: 0,
            current_time: 0,
            play_image: new_icon(PLAY_ICON),
            stopped: true,
        }
    }

    fn player_message(&mut self, player_msg: PlayerMsg) {
        match player_msg {
            PlayerPlay => {
                self.model.stopped = false;
                self.set_play_icon(PAUSE_ICON);
            },
            PlayerStop => {
                self.set_play_icon(PLAY_ICON);
                self.model.stopped = true;
            },
            PlayerTime(time) => self.set_current_time(time),
        }
    }

    fn set_current_time(&mut self, time: u64) {
        self.model.current_time = time;
        self.model.adjustment.set_value(time as f64);
    }

    fn set_play_icon(&self, icon: &str) {
        self.model.play_image.set_from_file(format!("assets/{}.png", icon));
    }

    fn update(&mut self, event: Msg) {
        match event {
            Duration(duration) => {
                self.model.current_duration = duration;
                self.model.adjustment.set_upper(duration as f64);
            },
            MsgRecv(player_msg) => self.player_message(player_msg),
            Next => self.playlist.emit(NextSong),
            Open => self.open(),
            PlayPause =>  {
                if self.model.stopped {
                    self.playlist.emit(PlaySong);
                } else {
                    self.playlist.emit(PauseSong);
                    self.set_play_icon(PLAY_ICON);
                }
            },
            Previous => self.playlist.emit(PreviousSong),
            Quit => gtk::main_quit(),
            Remove => self.playlist.emit(RemoveSong),
            Save => {
                let file = show_save_dialog(&self.window);
                if let Some(file) = file {
                    self.playlist.emit(SaveSong(file));
                }
            },
            Started(pixbuf) => {
                self.set_play_icon(PAUSE_ICON);
                self.model.cover_visible = true;
                self.model.cover_pixbuf = pixbuf;
            },
            Stop => {
                self.set_current_time(0);
                self.model.current_duration = 0;
                self.playlist.emit(StopSong);
                self.model.cover_visible = false;
                self.set_play_icon(PLAY_ICON);
            },
        }
    }

    view! {
        #[name="window"]
        gtk::Window {
            title: "Rusic",
            gtk::Box {
                orientation: Vertical,
                #[name="toolbar"]
                gtk::Toolbar {
                    gtk::ToolButton {
                        icon_widget: &new_icon("document-open"),
                        clicked => Open,
                    },
                    gtk::ToolButton {
                        icon_widget: &new_icon("document-save"),
                        clicked => Save,
                    },
                    gtk::SeparatorToolItem {
                    },
                    gtk::ToolButton {
                        icon_widget: &new_icon("gtk-media-previous"),
                        clicked => Previous,
                    },
                    gtk::ToolButton {
                        icon_widget: &self.model.play_image,
                        clicked => PlayPause,
                    },
                    gtk::ToolButton {
                        icon_widget: &new_icon("gtk-media-stop"),
                        clicked => Stop,
                    },
                    gtk::ToolButton {
                        icon_widget: &new_icon("gtk-media-next"),
                        clicked => Next,
                    },
                    gtk::SeparatorToolItem {
                    },
                    gtk::ToolButton {
                        icon_widget: &new_icon("remove"),
                        clicked => Remove,
                    },
                    gtk::SeparatorToolItem {
                    },
                    gtk::ToolButton {
                        icon_widget: &new_icon("gtk-quit"),
                        clicked => Quit,
                    },
                },
                #[name="playlist"]
                Playlist {
                    PlayerMsgRecv(ref player_msg) => MsgRecv(player_msg.clone()),
                    SongDuration(duration) => Duration(duration),
                    SongStarted(ref pixbuf) => Started(pixbuf.clone()),
                },
                gtk::Image {
                    from_pixbuf: self.model.cover_pixbuf.as_ref(),
                    visible: self.model.cover_visible,
                },
                gtk::Box {
                    orientation: Horizontal,
                    spacing: 10,
                    gtk::Scale(Horizontal, &self.model.adjustment) {
                        draw_value: false,
                        hexpand: true,
                    },
                    gtk::Label {
                        text: &millis_to_minutes(self.model.current_time),
                    },
                    gtk::Label {
                        text: "/",
                    },
                    gtk::Label {
                        margin_right: 10,
                        text: &millis_to_minutes(self.model.current_duration),
                    },
                },
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

impl App {
    fn open(&self) {
        let file = show_open_dialog(&self.window);
        if let Some(file) = file {
            let ext = file.extension().map(|ext| ext.to_str().unwrap().to_string());
            if let Some(ext) = ext {
                match ext.as_str() {
                    "mp3" => self.playlist.emit(AddSong(file)),
                    "m3u" => self.playlist.emit(LoadSong(file)),
                    extension => {
                        let dialog = MessageDialog::new(Some(&self.window), DialogFlags::empty(), MessageType::Error,
                        ButtonsType::Ok, &format!("Cannot open file with extension .{}", extension));
                        dialog.run();
                        dialog.destroy();
                    },
                }
            }
        }
    }

}

fn millis_to_minutes(millis: u64) -> String {
    let mut seconds = millis / 1_000;
    let minutes = seconds / 60;
    seconds %= 60;
    format!("{}:{:02}", minutes, seconds)
}

fn new_icon(icon: &str) -> Image {
    Image::new_from_file(format!("assets/{}.png", icon))
}

fn show_open_dialog(parent: &Window) -> Option<PathBuf> {
    let mut file = None;
    let dialog = FileChooserDialog::new(Some("Select an MP3 audio file"), Some(parent), FileChooserAction::Open);

    let mp3_filter = FileFilter::new();
    mp3_filter.add_mime_type("audio/mp3");
    mp3_filter.set_name("MP3 audio file");
    dialog.add_filter(&mp3_filter);

    let m3u_filter = FileFilter::new();
    m3u_filter.add_mime_type("audio/x-mpegurl");
    m3u_filter.set_name("M3U playlist file");
    dialog.add_filter(&m3u_filter);

    dialog.add_button("Cancel", RESPONSE_CANCEL);
    dialog.add_button("Accept", RESPONSE_ACCEPT);
    let result = dialog.run();
    if result == RESPONSE_ACCEPT {
        file = dialog.get_filename();
    }
    dialog.destroy();
    file
}

fn show_save_dialog(parent: &Window) -> Option<PathBuf> {
    let mut file = None;
    let dialog = FileChooserDialog::new(Some("Choose a destination M3U playlist file"), Some(parent), FileChooserAction::Save);
    let filter = FileFilter::new();
    filter.add_mime_type("audio/x-mpegurl");
    filter.set_name("M3U playlist file");
    dialog.set_do_overwrite_confirmation(true);
    dialog.add_filter(&filter);
    dialog.add_button("Cancel", RESPONSE_CANCEL);
    dialog.add_button("Save", RESPONSE_ACCEPT);
    let result = dialog.run();
    if result == RESPONSE_ACCEPT {
        file = dialog.get_filename();
    }
    dialog.destroy();
    file
}

fn to_millis(duration: Duration) -> u64 {
    duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1_000_000
}
