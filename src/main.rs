#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#[macro_use]
extern crate enum_display_derive;

extern crate cpal;

mod leaprust;
mod lrcpal;
mod lrviz;


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
use lrviz::{AppData, FrameUpdate, CustomView };

use std::mem;
use std::time::{self, SystemTime};

use winit::event_loop::EventLoopProxy;
use vizia::prelude::{
    Application,
    Event,
    HStack,
    Label,
    LayoutModifiers,
    Model,
    Percentage,
    Pixels,
    Stretch,
    VStack,
    WindowModifiers,
};





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
        proxy.send_event(Event::new(FrameUpdate {})).expect("poop");
    }
}


fn main() {
    let frame = unsafe { blank_frame() };
    /* The frame communicates 1-way from the controller to the cpal thread */
    let app = Application::new(move |cx| {
        // Build the model data into the tree
        AppData { frame: frame, timestamp: 0 }.build(cx);
        VStack::new(cx, |cx| {
            Label::new(cx, "Hello 1");
            HStack::new(cx , |cx| {
                //RadioButton::new(cx, );
            });
            CustomView::new(cx, AppData::timestamp).width(Percentage(100.0)).height(Percentage(100.0));
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
    let stream = lrcpal::set_up_cpal(frame);

    app.run();

    stream.pause().expect("Failed to pause stream");

    unsafe {
        remove_listener(controller);
        clean_up(controller, frame);
    }
}