#[macro_use]
extern crate enum_display_derive;
extern crate cpal;


mod leaprust;
mod lrcpal;
mod lrviz;
mod notefreq;

use symphonia_bundle_mp3::MpaReader;

use cpal::traits::StreamTrait;

use leaprust::{
    add_listener,
    blank_frame,
    clean_up,
    get_controller,
    LeapRustController,
    LeapRustEnv,
    LeapRustFrame,
    remove_listener,
};

use lrviz::{AppData, AppEvent, CustomView};
use lrcpal::{NoteShape, set_up_cpal};
use rtrb::RingBuffer;

use std::mem;
use std::time::{self, SystemTime};

use vizia::prelude::{
    Application,
    Button,
    Event,
    HStack,
    Label,
    LayoutModifiers,
    Model,
    Percentage,
    Pixels,
    RadioButton,
    Stretch,
    VStack,
    WindowModifiers, EmitContext,
};
use winit::event_loop::EventLoopProxy;

const STYLE: &str = r#"

    button {
        border-radius: 3px;
        child-space: 1s;
    }
    hstack {
        child-space: 1s;
        col-between: 20px;
    }
"#;


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
        proxy.send_event(Event::new(AppEvent::FrameUpdate)).expect("poop");
    }
}

fn main() {
    let frame = unsafe { blank_frame() };
    let (mut prod, mut cons) = RingBuffer::<AppEvent>::new(5);
    /* The frame communicates 1-way from the controller to the cpal thread */
    let app = Application::new(move |cx| {
        cx.add_theme(STYLE);
        // Build the model data into the tree
        AppData{
            frame: frame,
            timestamp: 0,
            placeholder: false,
            note_shape: NoteShape::SineSquared,
            ring_buf: prod
        }.build(cx);
        VStack::new(cx, |cx| {
            HStack::new(cx , |cx| {
                Button::new(cx, |cx| cx.emit(AppEvent::SetShape(NoteShape::Sine)), |cx| Label::new(cx, "Sin"));
                Button::new(cx, |cx| cx.emit(AppEvent::SetShape(NoteShape::SineSquared)), |cx| Label::new(cx, "S^2"));
                Button::new(cx, |cx| cx.emit(AppEvent::SetShape(NoteShape::Triangle)), |cx| Label::new(cx, "Tri"));
                Button::new(cx, |cx| cx.emit(AppEvent::SetShape(NoteShape::Saw)), |cx| Label::new(cx, "Saw"));
            })
                .child_space(Stretch(1.0))
                .col_between(Pixels(4.0));
            CustomView::new(cx, AppData::timestamp)
                .width(Percentage(99.0))
                .height(Percentage(50.0));
        })
        .child_space(Stretch(1.0))
        .col_between(Pixels(50.0));
    })
    .title("Counter")
    .inner_size((1024, 768));

    let event_proxy = Box::new(app.get_proxy());
    let mut env = unsafe { LeapRustEnv {
        frame: frame,
        event_proxy: mem::transmute(&event_proxy),
    }};
    let controller: *mut LeapRustController;
    unsafe {
        controller = get_controller(&mut env, Some(callback));
        add_listener(controller);
    }
    let stream = set_up_cpal(frame, cons);

    app.run();

    stream.pause().expect("Failed to pause stream");

    unsafe {
        remove_listener(controller);
        clean_up(controller, frame);
    }
}
