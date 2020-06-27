use std::cell::Cell;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam::sync::SegQueue;
use futures::{AsyncSink, Sink};
use futures::sync::mpsc::UnboundedSender;
use pulse_simple::Playback;

use mp3::Mp3Decoder;
use playlist::PlayerMsg::{
    self,
    PlayerPlay,
    PlayerStop,
    PlayerTime,
};
use self::Action::*;

const BUFFER_SIZE: usize = 1000;
const DEFAULT_RATE: u32 = 44100;

enum Action {
    Load(PathBuf),
    Stop,
}

#[derive(Clone)]
struct EventLoop {
    condition_variable: Arc<(Mutex<bool>, Condvar)>,
    queue: Arc<SegQueue<Action>>,
    playing: Arc<Mutex<bool>>,
}

pub struct Player {
    event_loop: EventLoop,
    paused: Cell<bool>,
    tx: UnboundedSender<PlayerMsg>,
}

impl Player {
    pub(crate) fn new(tx: UnboundedSender<PlayerMsg>) -> Self {
        let condition_variable = Arc::new((Mutex::new(false), Condvar::new()));
        let event_loop = EventLoop {
            condition_variable: condition_variable.clone(),
            queue: Arc::new(SegQueue::new()),
            playing: Arc::new(Mutex::new(false)),
        };

        {
            let mut tx = tx.clone();
            let event_loop = event_loop.clone();
            thread::spawn(move || {
                let block = || {
                    let (ref lock, ref condition_variable) = *condition_variable;
                    let mut started = lock.lock().unwrap();
                    *started = false;
                    while !*started {
                        started = condition_variable.wait(started).unwrap();
                    }
                };

                let mut buffer = [[0; 2]; BUFFER_SIZE];
                let mut playback = Playback::new("MP3", "MP3 Playback", None, DEFAULT_RATE);
                let mut source = None;
                loop {
                    if let Some(action) = event_loop.queue.try_pop() {
                        match action {
                            Load(path) => {
                                let file = File::open(path).unwrap();
                                source = Some(Mp3Decoder::new(BufReader::new(file)).unwrap());
                                let rate = source.as_ref().map(|source| source.samples_rate()).unwrap_or(DEFAULT_RATE);
                                playback = Playback::new("MP3", "MP3 Playback", None, rate);
                                send(&mut tx, PlayerPlay);
                            },
                            Stop => {
                                source = None;
                            },
                        }
                    } else if *event_loop.playing.lock().unwrap() {
                        let mut written = false;
                        if let Some(ref mut source) = source {
                            let size = iter_to_buffer(source, &mut buffer);
                            if size > 0 {
                                send(&mut tx, PlayerTime(source.current_time()));
                                playback.write(&buffer[..size]);
                                written = true;
                            }
                        }

                        if !written {
                            send(&mut tx, PlayerStop);
                            *event_loop.playing.lock().unwrap() = false;
                            source = None;
                            block();
                        }
                    } else {
                        block();
                    }
                }
            });
        }

        Player {
            event_loop,
            paused: Cell::new(false),
            tx,
        }
    }

    pub fn compute_duration<P: AsRef<Path>>(path: P) -> Option<Duration> {
        let file = File::open(path).unwrap();
        Mp3Decoder::compute_duration(BufReader::new(file))
    }

    fn emit(&self, action: Action) {
        self.event_loop.queue.push(action);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.get()
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) {
        let pathbuf = path.as_ref().to_path_buf();
        self.emit(Load(pathbuf));
        self.set_playing(true);
    }

    pub fn pause(&mut self) {
        self.paused.set(true);
        self.send(PlayerStop);
        self.set_playing(false);
    }

    pub fn resume(&mut self) {
        self.paused.set(false);
        self.send(PlayerPlay);
        self.set_playing(true);
    }

    fn set_playing(&self, playing: bool) {
        *self.event_loop.playing.lock().unwrap() = playing;
        let (ref lock, ref condition_variable) = *self.event_loop.condition_variable;
        let mut started = lock.lock().unwrap();
        *started = playing;
        if playing {
            condition_variable.notify_one();
        }
    }

    pub fn stop(&mut self) {
        self.paused.set(false);
        self.send(PlayerTime(0));
        self.send(PlayerStop);
        self.emit(Stop);
        self.set_playing(false);
    }

    fn send(&mut self, msg: PlayerMsg) {
        send(&mut self.tx, msg);
    }
}

fn iter_to_buffer<I: Iterator<Item=i16>>(iter: &mut I, buffer: &mut [[i16; 2]; BUFFER_SIZE]) -> usize {
    let mut iter = iter.take(BUFFER_SIZE);
    let mut index = 0;
    while let Some(sample1) = iter.next() {
        if let Some(sample2) = iter.next() {
            buffer[index][0] = sample1;
            buffer[index][1] = sample2;
        }
        index += 1;
    }
    index
}

fn send(tx: &mut UnboundedSender<PlayerMsg>, msg: PlayerMsg) {
    if let Ok(AsyncSink::Ready) = tx.start_send(msg) {
        tx.poll_complete().unwrap();
    } else {
        eprintln!("Unable to send message to sender");
    }
}
