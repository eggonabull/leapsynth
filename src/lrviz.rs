use crate::leaprust::{LeapRustFrame, LeapRustBoneType_TYPE_INTERMEDIATE};
use crate::lrcpal::{NoteShape};

use vizia::vg;
use vizia::prelude::{
    Canvas,
    Context,
    DataContext,
    DrawContext,
    Event,
    EventContext,
    Handle,
    Lens,
    Model,
    View,
};
use rtrb::Producer;

#[derive(Copy, Clone)]
pub enum AppEvent {
    FrameUpdate,
    SetShape(NoteShape)
}


#[derive(Lens)]
pub struct AppData {
    pub timestamp: i32,
    pub frame: *mut LeapRustFrame,
    pub placeholder: bool,
    pub note_shape: NoteShape,
    pub ring_buf: Producer<AppEvent>
}

// Describe how the data can be mutated
impl Model for AppData {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::FrameUpdate => {
                self.timestamp = unsafe { (*(self.frame)).timestamp };
            },
            AppEvent::SetShape(shape) => {
                self.note_shape = *shape;
                println!("pushing event");
                self.ring_buf.push(*app_event).expect("Failed to push");
            }
        });
    }
}

pub struct CustomView { }

impl CustomView {
    pub fn new(cx: &mut Context, frame: impl Lens<Target = i32>) -> Handle<Self> {
        Self{ }
          .build(cx, |_|{})
          .bind(frame, |handle, _frame_lens| {
            handle.cx.need_redraw()
          })
    }
}

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


static mut finger_t: f32 = f32::NEG_INFINITY;
static mut finger_l: f32 = f32::INFINITY;
static mut finger_b: f32 = f32::INFINITY;
static mut finger_r: f32 = f32::NEG_INFINITY;

impl View for CustomView {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if let Some(app_data) = cx.data::<AppData>() {
            let frame = &unsafe {*(app_data.frame)};

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
            let ((l, t), (r, b)) = (bounds.top_left(), bounds.bottom_right());
            let coord_converter = LeapCoordConverter { t, l, b, r };

            // draw intermediate nuckle center
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

            // draw tip of fingers
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

            // blue trigger border
            let mut path = vg::Path::new();
            let (_, border_y) = coord_converter.convert(0f32, 200.0);
            path.move_to(l, border_y);
            path.line_to(r,  border_y);
            canvas.stroke_path(&mut path, &blue1);

            // boundary circles -- temporary
            let mut path = vg::Path::new();
            path.rect(l, t, r-l, b-t);
            canvas.stroke_path(&mut path, &green1);
        }
    }
}
