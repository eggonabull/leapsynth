use cpal::{Sample, SampleFormat, StreamConfig, Stream};
use cpal::traits::{DeviceTrait, HostTrait};
use crate::leaprust::{LeapRustVector, LeapRustFrame};
use crate::lrviz::AppEvent;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fmt;
use std::fmt::Display;
use std::sync::atomic::{AtomicPtr, Ordering};
use rtrb::Consumer;

use crate::notefreq;

#[derive(Debug, Clone, Copy, Display, PartialEq)]
pub enum NoteState {
    Rising,
    Steady,
    Dying,
    Dead
}

#[derive(Debug, Clone, Copy, Display, PartialEq, Eq, Hash)]
enum Finger {
    Thumb,
    Index,
    Middle,
    Ring,
    Little
}

#[derive(Debug, Clone, Copy, Display, PartialEq)]
pub enum NoteShape {
    Sine,
    SineSquared,
    Saw,
    Triangle
}

struct PlaybackWave {
    freq: f32,
    shape: NoteShape,
}

struct PlaybackSample {
    sample_def: Vec<f32>,
    sample_rate: f32
}

enum PlaybackType {
    wave(PlaybackWave),
    sample(PlaybackSample)
}

pub struct TriggerDefinition {
    notes: Vec<PlaybackType>,
}

pub struct State {
    selected_map: i32,
    freq_map: HashMap<i32, HashMap<Finger, TriggerDefinition>>,
    active_playback: Vec<Note>,
    retrigger: bool,
    sample_rate: u32,
    shape: NoteShape
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
            self.volume = self.volume * 0.99995 - 0.00000001;
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
            let delta = (position.x - self.position.x) / 1000.0;
            let note_freq = self.freq;
            let new_freq = note_freq * (1.0 + delta);
            self.position.x = position.x;
            self.target_freq = new_freq;
        }
    }
}


impl State {
    fn new(sample_rate: u32) -> State {
        let mut map: HashMap<i32, HashMap<Finger, TriggerDefinition>> = HashMap::new();
        let mut default_map = HashMap::new();
        default_map.insert(Finger::Thumb, TriggerDefinition{notes: vec!(PlaybackType::wave(notefreq::C_4))});
        default_map.insert(Finger::Index, notefreq::D_4);
        default_map.insert(Finger::Middle, notefreq::E_4);
        default_map.insert(Finger::Ring, notefreq::F_4);
        default_map.insert(Finger::Little, notefreq::G_4);
        map.insert(0, default_map);

        let mut second_map = HashMap::new();
        second_map.insert(Finger::Thumb, notefreq::E_4);
        second_map.insert(Finger::Index, notefreq::G_4);
        second_map.insert(Finger::Middle, notefreq::A_4);
        second_map.insert(Finger::Ring, notefreq::C_5);
        second_map.insert(Finger::Little, notefreq::E_5);
        map.insert(1, second_map);

        let state = State {
            active_playback: Vec::new(),
            sample_rate: sample_rate,
            freq_map: map,
            selected_map: 0,
            retrigger: false,
            shape: NoteShape::SineSquared
        };
        state
    }

    fn get_sample(&mut self, i: u32) -> f32 {
        let mut val = 0f32;
        for note in &mut self.active_playback {
            let note_val = note.getSample(self.sample_rate, i);
            val += note_val;
        }

        self.active_playback.retain(|x| x.should_retain());

        if i % 270000 == 0 && self.active_playback.len() > 0 {
            println!("Notes: {}", self.active_playback.len());
        }

        if val > 1.0 {
            val = 1.0;
        }
        if self.active_playback.len() > 0 {
            val
        } else {
            0 as f32
        }
    }

    fn add_note(&mut self, note: Note) {
        self.active_playback.push(note);
        self.active_playback.retain(|x| x.should_retain())
    }

    fn has_note(&self, finger: Finger) -> Option<usize> {
        let index = self.active_playback.iter()
            .position(|x| x.matches(finger));
        return index;
    }

    fn remove_note(&mut self, finger: Finger) {
        let index = self.active_playback.iter()
            .position(|x| x.matches(finger));
        if let Some(index) = index {
            self.active_playback[index].kill()
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

fn is_finger_active(frame: &LeapRustFrame, finger: Finger, fing_index: usize) -> bool {
    let bottom = if finger != Finger::Thumb { 200f32 } else { 190f32 };
    if frame.handCount == 0 {
        return false;
    }
    if frame.handCount == 1 && frame.hands[0].isLeft == 1 {
        return false;
    }
    let right_hand = if frame.handCount == 1 {
        &frame.hands[0]
    } else {
        if frame.hands[0].isLeft == 0 {
            &frame.hands[0]
        } else {
            &frame.hands[1]
        }
    };
    let should_be_present = frame.handCount > 0 &&
        right_hand.fingerCount > fing_index.try_into().unwrap() &&
        right_hand.fingers[fing_index].tipPosition.y < bottom;
    return should_be_present
}

fn handle_finger(frame: &LeapRustFrame, finger: Finger, notes: &mut State) {
    let fing_index = finger_to_usize(finger);
    let has_note = notes.has_note(finger);
    let should_be_present = is_finger_active(frame, finger, fing_index);
    let freq = notes.freq_map
        .get(&notes.selected_map).expect("poo")
        .get(&finger).expect("asdf");
    if has_note.is_none() && should_be_present {
        println!("adding {} with x {}", finger, frame.hands[0].fingers[fing_index].tipPosition.x);
        for playback_type in freq.notes {
            match playback_type {
                PlaybackType::wave(PlaybackWave { freq, shape }) => {
                    notes.add_note(Note {
                        shape: notes.shape,
                        finger,
                        freq: freq,
                        target_freq: freq,
            
                        state: NoteState::Rising,
                        volume: 0.0,
                        target_volume: 0.2,
            
                        position: frame.hands[0].fingers[fing_index].tipPosition,
                        phase: 0.0,
                    });
                },
                _ => {}
            }
        }
    } else if has_note.is_some() && !should_be_present {
        println!("removing {}", finger);
        notes.remove_note(finger);
    } else if has_note.is_some() && should_be_present {
        // check for bends
        let finger_position = frame.hands[0].fingers[fing_index].tipPosition;
        let note = &mut (notes.active_playback[has_note.unwrap()]);
        note.update_position(finger_position)
    }
}

fn read_and_play(frame_ptr: *mut LeapRustFrame, notes: &mut State) {
    let frame;
    unsafe {
        frame = &(*frame_ptr);
    }
    for hand_index in 0..frame.handCount {
        let hand = &frame.hands[hand_index as usize];
        if hand.isLeft == 0 {
            continue;
        }
        if hand.fingers[1].tipPosition.y < 200.0 {
            notes.selected_map = 1;
        } else if hand.fingers[0].tipPosition.y < 200.0 {
            notes.selected_map = 0;
        }
    }

    handle_finger(frame, Finger::Thumb, notes);
    handle_finger(frame, Finger::Index, notes);
    handle_finger(frame, Finger::Middle, notes);
    handle_finger(frame, Finger::Ring, notes);
    handle_finger(frame, Finger::Little, notes);
    //handle_finger(frame, 5, 1174.66f32, collector, notes);
}

pub fn set_up_cpal(frame: *mut LeapRustFrame, mut ring_buf: Consumer<AppEvent>) -> Stream {
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
    let mut state = State::new(config.sample_rate.0);
    
    let mut last_timestamp: i32 = 0;
    let aframe: AtomicPtr<LeapRustFrame> = AtomicPtr::new(frame);
    let create_audio_stream = move |data: &mut [f32], _cb: &cpal::OutputCallbackInfo| {
        let tframe = aframe.load(Ordering::Relaxed);
        if let Some(app_event) = ring_buf.pop().ok() {
            match app_event {
                AppEvent::SetShape(shape) => {
                    println!("popping event");
                    state.shape = shape;
                }
                _ => {}
            }
        }
        for sample in data.iter_mut() {
            let frame_stamp = unsafe { (*tframe).timestamp };
            if last_timestamp != frame_stamp {
                read_and_play(tframe, &mut state);
                last_timestamp = frame_stamp;
            }
            let val = state.get_sample(i);
            *sample = Sample::from(&val);
            i = i + 1;
            if state.active_playback.len() == 0 {
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
