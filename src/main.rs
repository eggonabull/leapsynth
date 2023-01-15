#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
extern crate cpal;

#[macro_use]
extern crate enum_display_derive;

use std::f32::consts::PI;
use std::fmt;
use std::fmt::Display;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread;
use std::time::{self, SystemTime};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig, Stream};

use vizia::prelude::*;



include!(concat!(env!("OUT_DIR"), "/bindings.rs"));


#[derive(Debug, Clone, Copy, Display, PartialEq)]
enum NoteState {
    Rising,
    Steady,
    Dying,
    Dead
}

#[derive(Debug, Clone, Copy, Display, PartialEq)]
enum Finger {
    Thumb,
    Index,
    Middle,
    Ring,
    Little
}

#[derive(Debug, Clone, Copy, Display, PartialEq)]
enum NoteShape {
    Sine,
    SineSquared,
    Saw,
    Triangle
}


#[derive(Debug, Copy, Clone)]
struct Note {
    finger: Finger,
    state: NoteState,
    shape: NoteShape,

    freq: f32,
    target_freq: f32,

    volume: f32,
    target_volume: f32,

    position: LeapRustVector,
    phase: f32,
}


impl Note {
    fn kill(&mut self) {
        self.state = NoteState::Dying;
    }

    fn should_retain(self) -> bool {
        return self.state != NoteState::Dead
    }

    fn matches(self, finger: Finger) -> bool {
        return self.finger == finger && self.state != NoteState::Dying && self.state != NoteState::Dead
    }

    fn getSample(self: &mut Self, sample_rate: u32, i: u32) -> f32 {
        if self.state == NoteState::Dead {
            return 0f32;
        }

        if self.state == NoteState::Rising {
            self.volume += 0.000002;
            if self.volume > self.target_volume {
                self.state = NoteState::Steady;
            }
        }

        if self.state == NoteState::Dying {
            self.volume -= 0.0000004;
            if self.volume < 0f32 {
                self.volume = 0f32;
                self.state = NoteState::Dead;
            }
        }

        let twopi = 2.0 * PI;
        let t = (i as f32 % (sample_rate as f32 * self.freq)) as f32 / sample_rate as f32;
        if self.target_freq != self.freq {
            self.phase = (self.phase + 2.0 * PI * (t * self.freq - self.target_freq * t)) % twopi;
            self.freq = self.target_freq;
        }
        let position = (2.0 * PI * t * self.freq) + self.phase;
        let val = position.sin();
        match self.shape {
            NoteShape::Sine => val * self.volume,
            NoteShape::SineSquared => val * val * val.signum() * self.volume,
            NoteShape::Saw => ((position % twopi / twopi) - 0.5) * self.volume,
            NoteShape::Triangle => {
                let pos = position % twopi / twopi;
                let result = if pos < 0.25 {
                    pos * 4.0 - 1.0
                } else if pos < 0.75 {
                    1.0 - (pos - 0.25) * 4.0
                } else {
                    (pos - 0.75) * 4.0 - 1.0
                };
                result * self.volume
            }
        }
    }


    fn update_position(&mut self, position: LeapRustVector) {
        if position.x != self.position.x {
            let delta = (position.x - self.position.x) / 5000.0;
            let note_freq = self.freq;
            let new_freq = note_freq * (1.0 + delta);
            self.target_freq = new_freq;
        }
    }
}

struct State {
    notes: Vec<Note>,
    sample_rate: u32
}

impl State {
    fn get_sample(&mut self, i: u32) -> f32 {
        let mut val = 0f32;
        for note in &mut self.notes {
            let note_val = note.getSample(self.sample_rate, i);
            val += note_val;
        }

        self.notes.retain(|x| x.should_retain());

        if i % 270000 == 0 && self.notes.len() > 0 {
            println!("Notes: {}", self.notes.len());
        }

        if val > 1.0 {
            val = 1.0;
        }
        if self.notes.len() > 0 {
            val
        } else {
            0 as f32
        }
    }

    fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.retain(|x| x.should_retain())
    }

    fn has_note(&self, finger: Finger) -> Option<usize> {
        let index = self.notes.iter().position(|x| x.matches(finger));
        return index;
    }

    fn remove_note(&mut self, finger: Finger) {
        let index = self.notes.iter().position(|x| x.matches(finger));
        if let Some(index) = index {
            self.notes[index].kill()
        }
    }
}

impl fmt::Display for LeapRustVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

fn finger_to_usize(finger: Finger) -> usize {
    return finger as usize;
}

fn handle_finger(frame: LeapRustFrame, finger: Finger, freq: f32, notes: &mut State) {
    let has_note = notes.has_note(finger);
    let fing_index = finger_to_usize(finger);
    let bottom = if finger != Finger::Thumb { 200f32 } else { 190f32 };
    let should_be_present = frame.handCount > 0 &&
        frame.hands[0].fingerCount > fing_index.try_into().unwrap() &&
        frame.hands[0].fingers[fing_index].tipPosition.y < bottom;
    if has_note.is_none() && should_be_present {
        println!("adding {} with x {}", finger, frame.hands[0].fingers[fing_index].tipPosition.x);
        notes.add_note(Note {
            shape: NoteShape::SineSquared,
            finger,
            freq: freq,
            target_freq: freq,

            state: NoteState::Rising,
            volume: 0.0,
            target_volume: 0.2,

            position: frame.hands[0].fingers[fing_index].tipPosition,
            phase: 0.0,
        });
    } else if has_note.is_some() && !should_be_present {
        println!("removing {}", finger);
        notes.remove_note(finger);
    } else if has_note.is_some() && should_be_present {
        // check for bends
        let finger_position = frame.hands[0].fingers[fing_index].tipPosition;
        let note = &mut (notes.notes[has_note.unwrap()]);
        note.update_position(finger_position)
    }
}

fn read_and_play(frame_ptr: *mut LeapRustFrame, notes: &mut State) {
    let frame;
    unsafe {
        frame = *frame_ptr;
    }
    handle_finger(frame, Finger::Thumb, 440f32, notes);
    handle_finger(frame, Finger::Index, 523.25f32, notes);
    handle_finger(frame, Finger::Middle, 659.25f32, notes);
    handle_finger(frame, Finger::Ring, 783.99f32, notes);
    handle_finger(frame, Finger::Little, 987.77f32, notes);
    //handle_finger(frame, 5, 1174.66f32, collector, notes);
}

static mut NUM_FRAMES: i32 = 0;
static mut FIFTY_FRAME_TIME: SystemTime = SystemTime::UNIX_EPOCH;

extern fn callback(env: *mut LeapRustEnv, frame_ptr: *mut LeapRustFrame) {
    unsafe {
        if NUM_FRAMES % 50 == 0 {
            let new_now = time::SystemTime::now();
            println!("frame {} delay {:?}", NUM_FRAMES, (new_now.duration_since(FIFTY_FRAME_TIME)));
            FIFTY_FRAME_TIME = new_now;
        }
        NUM_FRAMES += 1;
        *(*env).frame = *frame_ptr;
    }
}


fn set_up_cpal(frame: *mut LeapRustFrame) -> Stream {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available");
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let mut supported_configs_range = device.supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range.next()
        .expect("no supported config?!")
        .with_max_sample_rate();
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();
    let mut i = 0;
    let mut state = State { notes: Vec::new(), sample_rate: config.sample_rate.0 };
    let mut last_timestamp: i32 = 0;
    let aframe: AtomicPtr<LeapRustFrame> = AtomicPtr::new(frame);
    let create_audio_stream = move |data: &mut [f32], _cb: &cpal::OutputCallbackInfo| {
        let tframe = aframe.load(Ordering::Relaxed);
        for sample in data.iter_mut() {
            let frame_stamp = unsafe { (*tframe).timestamp };
            if last_timestamp != frame_stamp {
                read_and_play(tframe, &mut state);
                last_timestamp = frame_stamp;
            }
            let val = state.get_sample(i);
            *sample = Sample::from(&val);
            i = i + 1;
            if state.notes.len() == 0 {
                i = 0;
            }
        }
    };
    let stream = match sample_format {
        SampleFormat::F32 => panic!("f32"),
        SampleFormat::I16 => device.build_output_stream(&config, create_audio_stream, err_fn),
        SampleFormat::U16 => panic!("u32"),
    }.unwrap();
    stream
}

#[derive(Lens)]
pub struct AppData {
    count: i32,
}

// Define events to mutate the data
pub enum AppEvent {
    Increment,
}

// Describe how the data can be mutated
impl Model for AppData {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::Increment => {
                self.count += 1;
            }
        });
    }
}

fn main() {
    /* The frame communicates 1-way from the controller to the cpal thread */
    let frame = unsafe { blank_frame() };
    let mut env = LeapRustEnv {
        frame: frame,
    };
    let controller: *mut LeapRustController;
    unsafe {
        controller = get_controller(&mut env, Some(callback));
        add_listener(controller);
    }
    let stream = set_up_cpal(frame);

    Application::new(|cx| {
        // Build the model data into the tree
        AppData { count: 0 }.build(cx);

        HStack::new(cx, |cx| {
            // Declare a button which emits an event
            Button::new(cx, |cx| cx.emit(AppEvent::Increment), |cx| Label::new(cx, "Increment"));

            // Declare a label which is bound to part of the model, updating if it changes
            Label::new(cx, AppData::count).width(Pixels(50.0));
        })
        .child_space(Stretch(1.0))
        .col_between(Pixels(50.0));
    })
    .title("Counter")
    .inner_size((400, 100))
    .run();


    loop {
        thread::sleep(time::Duration::from_secs(5));
    }
    thread::sleep(time::Duration::from_secs(5));
    stream.pause();
    unsafe {
        remove_listener(controller);
        clean_up(controller, frame);
    }
}