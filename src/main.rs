#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
extern crate cpal;

#[macro_use]
extern crate enum_display_derive;

use std::f32::consts::PI;
use std::fmt;
use std::fmt::Display;
use std::mem;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::{self, SystemTime};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig, Stream};

use vizia::vg;
use winit::event_loop::EventLoopProxy;
use vizia::prelude::{
    Application,
    Button,
    Canvas,
    Context,
    Data,
    DrawContext,
    Event,
    EventContext,
    Handle,
    Label,
    LayoutModifiers,
    Lens,
    Model,
    Percentage,
    Pixels,
    Stretch,
    View,
    VStack,
    WindowModifiers, DataContext
};



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
            //self.volume -= 0.0000004;
            self.volume = self.volume * 0.99995;
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
    frame: LeapRustFrame,
}

// Describe how the data can be mutated
impl Model for AppData {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            LeapRustFrame {id, timestamp, handCount, hands } => {
                self.frame = LeapRustFrame {
                    id:*id,
                    timestamp:*timestamp,
                    handCount: *handCount,
                    hands: *hands
                };
            }
        });
    }
}

impl Data for LeapRustFrame {
    fn same(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}


struct CustomView { }

impl CustomView {
    pub fn new(cx: &mut Context, frame: impl Lens<Target = LeapRustFrame>) -> Handle<Self> {
        Self{ }
          .build(cx, |_|{})
          .bind(frame, |handle, _frame_lens| {
            handle.cx.need_redraw()
          })
    }
}

static mut finger_t: f32 = f32::NEG_INFINITY;
static mut finger_l: f32 = f32::INFINITY;
static mut finger_b: f32 = f32::INFINITY;
static mut finger_r: f32 = f32::NEG_INFINITY;



struct LeapCoordConverter {
    t: f32,
    l: f32,
    b: f32,
    r: f32
}

impl LeapCoordConverter {
    fn convert(&self, x: f32, y: f32) -> (f32, f32) {
        let std_leap_y_max = 650f32;
        let std_leap_y_min = 10f32;
        let std_leap_x_max = 330f32;
        let std_leap_x_min = -420f32;
        let std_leap_height = std_leap_y_max - std_leap_y_min;
        let std_leap_width = std_leap_x_max - std_leap_x_min;

        let vwidth = self.r - self.l;
        let vheight = self.b - self.t;

        let y_1 = (std_leap_y_max - y) * vheight / std_leap_height + self.t;
        let x_1 = (x - std_leap_x_min) * vwidth / std_leap_width + self.l;

        (x_1, y_1)
    }
}

impl View for CustomView {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if let Some(app_data) = cx.data::<AppData>() {
            let frame = app_data.frame;

            // START - blurb to get leap finger position bounds
            unsafe {
                for hand_index in 0..frame.handCount {
                    let hand = frame.hands[hand_index as usize];
                    for finger_index in 0..hand.fingerCount {
                        let fingertip = hand.fingers[finger_index as usize].tipPosition;
                        if fingertip.x > finger_r { finger_r = fingertip.x}
                        if fingertip.x < finger_l { finger_l = fingertip.x}
                        if fingertip.y > finger_t { finger_t = fingertip.y}
                        if fingertip.y < finger_b { finger_b = fingertip.y}
                    }
                }
                //println!("{} {} {} {}", finger_t, finger_l, finger_b, finger_r);
            }
            // END - blurb to get leap finger position bounds

            let red1 = vg::Paint::color(vg::Color::rgb(200, 50, 50));
            let red2 = vg::Paint::color(vg::Color::rgb(200, 100, 100));
            let blue1 = vg::Paint::color(vg::Color::rgb(50, 50, 200));
            let green1 = vg::Paint::color(vg::Color::rgb(50, 100, 50));
            let green2 = vg::Paint::color(vg::Color::rgb(50, 150, 50));
            let green3 = vg::Paint::color(vg::Color::rgb(50, 200, 50));
            let green4 = vg::Paint::color(vg::Color::rgb(100, 200, 100));


            let bounds = cx.bounds();
            let ((t, l), (b, r)) = (bounds.top_left(), bounds.bottom_right());
            let coord_converter = LeapCoordConverter { t, l, b, r };

            // draw first (from tip) nuckle
            let mut path = vg::Path::new();
            path.move_to(t, l);
            for hand_index in 0..frame.handCount {
                let hand = frame.hands[hand_index as usize];
                for finger_index in 0..hand.fingerCount {
                    let finger = hand.fingers[finger_index as usize];
                    for bone in finger.bones {
                        //println!("bone type {}", bone.type_);
                        if bone.type_== LeapRustBoneType_TYPE_INTERMEDIATE {
                            let (x, y) = coord_converter.convert(bone.center.x, bone.center.y);
                            path.circle(x, y, 10.0);
                        }
                    }
                }
            }
            canvas.fill_path(&mut path, &red2);

            let mut path = vg::Path::new();
            path.move_to(t, l);
            for hand_index in 0..frame.handCount {
                let hand = frame.hands[hand_index as usize];
                for finger_index in 0..hand.fingerCount {
                    let finger = hand.fingers[finger_index as usize];
                    let (x, y) = coord_converter.convert(finger.tipPosition.x, finger.tipPosition.y);
                    path.circle(x, y, 10.0);
                }
            }
            canvas.fill_path(&mut path, &red1);
            let mut path = vg::Path::new();
            let (_, border_y) = coord_converter.convert(0f32, 200.0);
            path.move_to(l, border_y);
            path.line_to(r,  border_y);
            canvas.stroke_path(&mut path, &blue1);

            let mut path = vg::Path::new();
            path.circle(l, t, 10.0);
            canvas.fill_path(&mut path, &green1);

            let mut path = vg::Path::new();
            path.circle(r, t, 10.0);
            canvas.fill_path(&mut path, &green2);

            let mut path = vg::Path::new();
            path.circle(l, b, 10.0);
            canvas.fill_path(&mut path, &green3);

            let mut path = vg::Path::new();
            path.circle(r, b, 10.0);
            canvas.fill_path(&mut path, &green4);
        }
    }
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
        let proxy: &Box<EventLoopProxy<Event>> = mem::transmute((*env).event_proxy);
        proxy.send_event(Event::new(*frame_ptr)).expect("poop");
    }
}


fn main() {
    let frame = unsafe { blank_frame() };
    let frame2 = unsafe { blank_frame() };
    /* The frame communicates 1-way from the controller to the cpal thread */
    let app = Application::new(move |cx| {
        // Build the model data into the tree
        AppData { frame: unsafe{*frame2}}.build(cx);
        VStack::new(cx, |cx| {
            Label::new(cx, "Hello 1");
            CustomView::new(cx, AppData::frame).width(Percentage(100.0)).height(Percentage(100.0));
        })
        .child_space(Stretch(1.0))
        .col_between(Pixels(50.0));
    })
    .title("Counter")
    .inner_size((1024, 768));

    let event_proxy = Box::new(app.get_proxy());
    let mut env = unsafe { LeapRustEnv {
        frame: frame,
        event_proxy: mem::transmute(&event_proxy)
    }};
    let controller: *mut LeapRustController;
    unsafe {
        controller = get_controller(&mut env, Some(callback));
        add_listener(controller);
    }
    let stream = set_up_cpal(frame);

    app.run();

    stream.pause().expect("Failed to pause stream");

    unsafe {
        remove_listener(controller);
        clean_up(controller, frame);
    }
}