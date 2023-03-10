use cpal::{Sample, SampleFormat, StreamConfig, Stream};
use cpal::traits::{DeviceTrait, HostTrait};
use crate::leaprust::{LeapRustVector, LeapRustFrame};
use crate::lrviz::AppEvent;
use std::collections::HashMap;
use std::f32::NEG_INFINITY;
use std::f32::consts::PI;
use std::fmt;
use std::fmt::Display;
use std::sync::atomic::{AtomicPtr, Ordering};
use rtrb::Consumer;
use std::fs::File;
use std::path::Path;
use std::cmp::Ordering as CmpOrdering;

use symphonia::core::io::MediaSourceStream;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlaybackWave {
    freq: f32,
    target_freq: f32,
    shape: NoteShape,
    phase: f32,
}

impl PlaybackWave {
    fn new(freq: f32, shape: NoteShape) -> PlaybackWave {
        PlaybackWave {
            freq: freq,
            target_freq: freq,
            shape: shape,
            phase: 0f32
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PlaybackSample {
    sample_def: Vec<f32>,
    freq: u32,
    target_freq: u32,
    phase: f32,
    first_sample: bool,
    adjustment: u32,
    loop_start: Option<usize>,
    loop_end: Option<usize>
}


fn yield_slices<'a>(v: &'a [f32]) -> Vec<&'a[f32]> {
    let mut slices = vec![];
    let mut start = 0;
    let mut current_direction = CmpOrdering::Greater;
    let sum_size = 10;
    for i in sum_size..v.len() {
        let direction = (v[i-(sum_size/2 - 1)..i].iter().sum::<f32>()).partial_cmp(&v[i-(sum_size - 1)..i-(sum_size/2)].iter().sum::<f32>()).unwrap();
        if direction != current_direction {
            slices.push(&v[start..i]);
            start = i;
            current_direction = direction;
        }
    }
    slices.push(&v[start..]);
    slices
}


fn find_sustain_sample_bounds(signal: &Vec<f32>) -> (Option<usize>, Option<usize>) {
    let mut peak_amplitude = std::f32::NEG_INFINITY;
    let mut peak_index = 0 as usize;
    for (i, amplitude) in signal.iter().cloned().enumerate() {
        if amplitude > peak_amplitude {
            peak_index = i;
            peak_amplitude = amplitude;
        }
    }
    let slices = yield_slices(signal);
    for slice in slices {
        println!("slice len {} slice[0] {} slice[len-1] {}", slice.len(), slice[0], slice[slice.len() - 1]);
    }
    let threshold = peak_amplitude * 0.1;
    let mut first_index = None;
    let mut last_index = None;
    for (i, amplitude) in signal[peak_index..].iter().cloned().enumerate() {
        if amplitude >= peak_amplitude - threshold {
            first_index = Some(i + peak_index);
        } else {
            break;
        }
    }
    if let Some(first_index) = first_index {
        for (i, amplitude) in signal[(first_index+1)..].iter().cloned().enumerate() {
            if amplitude >= peak_amplitude - threshold {
                last_index = Some(i + first_index);
            }
        }
    }
    println!("peak index {} first_index {:?} last_index {:?} len {:?}", peak_index, first_index, last_index, signal.len());
    ///panic!("ahhhhh");
    (first_index, last_index)
}


impl PlaybackSample {
    fn new(sample_def: Vec<f32>, freq: u32) -> PlaybackSample {
        let (start, end) = find_sustain_sample_bounds(&sample_def);
        PlaybackSample {
            sample_def: sample_def,
            target_freq: freq,
            freq: freq,
            phase: 0f32,
            adjustment: 0u32,
            first_sample: true,
            loop_start: start,
            loop_end: end
        }
    }
}

trait PlaybackTypeItem {
    fn get_sample(&mut self, sample_rate: u32, i: u32) -> f32;
    fn adjust_freq(&mut self, mult: f32);
}


impl PlaybackTypeItem for PlaybackSample {
    fn get_sample(&mut self, sample_rate: u32, i: u32) -> f32 {
        // if self.target_freq != self.freq || self.first_sample{
        //     let old_index = if self.first_sample {
        //         println!("self.first_sample");
        //         self.first_sample = false;
        //         0f32
        //     } else {
        //         (i as f32 * (self.freq as f32 / sample_rate as f32) as f32) + self.phase
        //     };
        //     let new_index = i as f32 * (self.target_freq as f32 / sample_rate as f32) as f32;
        //     self.freq = self.target_freq;
        //     self.phase = old_index - new_index;
        // }
        let mut sample_index = ((i - self.adjustment) as f32 * (self.freq as f32 / sample_rate as f32) as f32) + self.phase;
        let mut show_debug = false;
        if let (Some(loop_start), Some(loop_end)) = (self.loop_start, self.loop_end) {
            if sample_index > loop_end as f32 {
                println!("current index {} loop_start {} loop_end {} phase {}", sample_index, loop_start, loop_end, self.phase);
                let adjustment_incr = (loop_end as u32 - loop_start as u32) as f32 *  sample_rate as f32 / self.freq as f32;
                self.adjustment += adjustment_incr as u32;
                let new_index = ((i - self.adjustment) as f32 * (self.freq as f32 / sample_rate as f32) as f32) + self.phase;
                sample_index = new_index;
                show_debug = true;
            }
        }

        let first_index = (sample_index as u32 % self.sample_def.len() as u32) as usize;
        let second_index = ((sample_index as u32 + 1) % self.sample_def.len() as u32) as usize;
        let second_weight = (sample_index - sample_index.floor());
        let first_weight = 1.0 - second_weight;

        let raw_sample_value =
            self.sample_def[first_index] as f32 * first_weight +
            self.sample_def[second_index] as f32 * second_weight
        ;
        let result = raw_sample_value as f32 * 10.0;
        if i % 81049 == 0 || show_debug {
            println!(
                "i: {}, Freq: {}, Sample rate: {}, Sample index: {}, data len: {}, Raw Val: {}, result: {}",
                i,
                self.freq,
                sample_rate,
                sample_index,
                self.sample_def.len(),
                raw_sample_value,
                result
            );
        }
        result
    }

    fn adjust_freq(&mut self, mult: f32) {
        self.target_freq = (self.target_freq as f32 * mult).round() as u32;
    }
}

impl PlaybackTypeItem for PlaybackWave {
    fn get_sample(&mut self, sample_rate: u32, i: u32) -> f32 {
        let twopi = 2.0 * PI;
        let t = (i as f32 % (sample_rate as f32 * self.freq)) as f32 / sample_rate as f32;
        if self.target_freq != self.freq {
            self.phase = (self.phase + 2.0 * PI * (t * self.freq - self.target_freq * t)) % twopi;
            self.freq = self.target_freq;
        }
        let position = (2.0 * PI * t * self.freq) + self.phase;
        let val = position.sin();
        match self.shape {
            NoteShape::Sine => val,
            NoteShape::SineSquared => val * val * val.signum(),
            NoteShape::Saw => (position % twopi / twopi) - 0.5,
            NoteShape::Triangle => {
                let pos = position % twopi / twopi;
                let result = if pos < 0.25 {
                    pos * 4.0 - 1.0
                } else if pos < 0.75 {
                    1.0 - (pos - 0.25) * 4.0
                } else {
                    (pos - 0.75) * 4.0 - 1.0
                };
                result
            }
        }
    }

    fn adjust_freq(&mut self, mult: f32) {
        self.target_freq = self.target_freq * mult;
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlaybackType {
    Wave(PlaybackWave),
    Sample(PlaybackSample)
}

impl PlaybackType {
    fn get_sample(&mut self, sample_rate: u32, i: u32) -> f32 {
        match self {
            PlaybackType::Wave(x) => x.get_sample(sample_rate, i),
            PlaybackType::Sample(x) => x.get_sample(sample_rate, i),
        }
    }

    fn adjust_freq(&mut self, mult: f32) {
        match self {
            PlaybackType::Wave(x) => x.adjust_freq(mult),
            PlaybackType::Sample(x) => x.adjust_freq(mult),
        }
    }
}


#[derive(Debug, Clone)]
pub struct TriggerDefinition {
    notes: Vec<PlaybackType>,
}

impl TriggerDefinition {
    fn get_sample(&mut self, sample_rate: u32, i: u32) -> f32 {
        let mut sum = 0f32;
        for note in &mut self.notes {
            sum += note.get_sample(sample_rate, i);
        }
        sum
    }
}

pub struct State {
    selected_map: i32,
    freq_map: HashMap<i32, HashMap<Finger, TriggerDefinition>>,
    active_playback: Vec<Note>,
    retrigger: bool,
    sample_rate: u32,
    shape: NoteShape
}


#[derive(Debug, Clone)]
struct Note {
    finger: Finger,
    state: NoteState,
    volume: f32,
    target_volume: f32,
    position: LeapRustVector,
    phase: f32,

    trigger: TriggerDefinition,
}


impl Note {
    fn kill(&mut self) {
        self.state = NoteState::Dying;
    }

    fn should_retain(&self) -> bool {
        return self.state != NoteState::Dead
    }

    fn matches(&self, finger: Finger) -> bool {
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

        self.trigger.get_sample(sample_rate, i) * self.volume
    }

    fn update_position(&mut self, position: LeapRustVector) {
        if position.x != self.position.x {
            let delta = (position.x - self.position.x) / 1000.0;
            let multiplier = (1.0 + delta);
            for wave in &mut self.trigger.notes {
                wave.adjust_freq(multiplier);
            }
            self.position.x = position.x;
        }
    }
}

fn file_to_sample(path: &str) -> PlaybackSample {
    let file = Box::new(File::open(Path::new(path)).expect(&format!("Couldn't open path {}", path)));
    let mss = MediaSourceStream::new(file, Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();
    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts).expect("unsupported format");
    let mut format = probed.format;
    let track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs().make(
        &track.codec_params,
        &decoder_opts
    ).unwrap();
    let mut joined_data = Vec::new();
    let mut spec_freq = 0;
    while let Some(packet) = format.next_packet().ok() {
        let decoded = decoder.decode(&packet).expect("Unable to decode packet");
        let spec = *decoded.spec();
        spec_freq = spec.rate;
        let mut samples = SampleBuffer::new(decoded.frames() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        for frame in samples.samples().chunks(spec.channels.count()) {
            for (chan, sample) in frame.iter().enumerate() {
                joined_data.push(*sample);
            }
        }
    }

    let sample = PlaybackSample::new(
        joined_data,
        spec_freq
    );
    sample
}


impl State {
    fn new(sample_rate: u32) -> State {
        let mut map: HashMap<i32, HashMap<Finger, TriggerDefinition>> = HashMap::new();
        let mut default_map = HashMap::new();
        default_map.insert(Finger::Thumb, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::C_4, NoteShape::SineSquared))
        )});
        default_map.insert(Finger::Index, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::D_4, NoteShape::SineSquared))
        )});
        default_map.insert(Finger::Middle, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::E_4, NoteShape::SineSquared))
        )});
        default_map.insert(Finger::Ring, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::F_4, NoteShape::SineSquared))
        )});
        default_map.insert(Finger::Little, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::G_4, NoteShape::SineSquared))
        )});
        map.insert(0, default_map);

        let mut second_map = HashMap::new();
        second_map.insert(Finger::Thumb, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::C_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::E_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::G_4, NoteShape::SineSquared))
        )});
        second_map.insert(Finger::Index, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::D_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::F_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::A_4, NoteShape::SineSquared))
        )});
        second_map.insert(Finger::Middle, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::E_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::G_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::B_4, NoteShape::SineSquared))
        )});
        second_map.insert(Finger::Ring, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::F_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::A_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::C_5, NoteShape::SineSquared))
        )});
        second_map.insert(Finger::Little, TriggerDefinition{notes: vec!(
            PlaybackType::Wave(PlaybackWave::new(notefreq::G_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::B_4, NoteShape::SineSquared)),
            PlaybackType::Wave(PlaybackWave::new(notefreq::D_5, NoteShape::SineSquared))
        )});
        map.insert(1, second_map);

        let mut third_map = HashMap::new();
        third_map.insert(Finger::Thumb, TriggerDefinition{notes: vec!(
            PlaybackType::Sample(file_to_sample("/home/drew/Downloads/Strings/violin/violin_A4_1_fortissimo_arco-normal.mp3"))
        )});
        third_map.insert(Finger::Index, TriggerDefinition{notes: vec!(
            PlaybackType::Sample(file_to_sample("/home/drew/Downloads/Strings/violin/violin_B4_1_fortissimo_arco-normal.mp3"))
        )});
        third_map.insert(Finger::Middle, TriggerDefinition{notes: vec!(
            PlaybackType::Sample(file_to_sample("/home/drew/Downloads/Strings/violin/violin_Cs5_1_fortissimo_arco-normal.mp3"))
        )});
        third_map.insert(Finger::Ring, TriggerDefinition{notes: vec!(
            PlaybackType::Sample(file_to_sample("/home/drew/Downloads/Strings/violin/violin_E5_1_fortissimo_arco-normal.mp3"))
        )});
        third_map.insert(Finger::Little, TriggerDefinition{notes: vec!(
            PlaybackType::Sample(file_to_sample("/home/drew/Downloads/Strings/violin/violin_Fs5_1_fortissimo_arco-normal.mp3"))
        )});
        map.insert(2, third_map);


        let state = State {
            active_playback: Vec::new(),
            sample_rate: sample_rate,
            freq_map: map,
            selected_map: 2,
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
    let trigger_def = notes.freq_map
        .get(&notes.selected_map).expect("poo")
        .get(&finger).expect("asdf");
    if has_note.is_none() && should_be_present {
        println!("adding {} with x {}", finger, frame.hands[0].fingers[fing_index].tipPosition.x);
        notes.add_note(Note {
            trigger: trigger_def.clone(),

            finger,

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
        if hand.fingers[2].tipPosition.y < 200.0 {
            notes.selected_map = 2;
        } else if hand.fingers[3].tipPosition.y < 200.0 {
            notes.selected_map = 1;
        } else if hand.fingers[4].tipPosition.y < 200.0 {
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
