#![recursion_limit = "1024"]
use std;
use cpp::{cpp, cpp_class};
use std::thread;
use std::time::Duration;


cpp!{{
    #include <iostream>
    #include "memory"
    #include "Leap.h"
}}

cpp!{{
    struct CallbackPtr { void *a, *b; };
    class SampleListener : public Leap::Listener {
        protected:
        void onInit(const Leap::Controller& controller) {
            std::cout << "Initialized" << std::endl;
        }

        void onConnect(const Leap::Controller& controller) {
            std::cout << "Connected" << std::endl;
            //   controller.enableGesture(Gesture::TYPE_CIRCLE);
            //   controller.enableGesture(Gesture::TYPE_KEY_TAP);
            //   controller.enableGesture(Gesture::TYPE_SCREEN_TAP);
            //   controller.enableGesture(Gesture::TYPE_SWIPE);
        }

        void onDisconnect(const Leap::Controller& controller) {
            // Note: not dispatched when running in a debugger.
            std::cout << "Disconnected" << std::endl;
        }

        void onExit(const Leap::Controller& controller) {
            std::cout << "Exited" << std::endl;
        }

        void onFrame(const Leap::Controller& controller) {
            // Get the most recent frame and report some basic information
            std::cout << "onFrame" << std::endl;
            const Leap::Frame leapFrame = controller.frame();
            rust!(BleepBloop [leapFrame: &LeapFrame as "Leap::Frame"] {
                println!("This is rust");
            });
        }

        void onDeviceChange(const Leap::Controller& controller) {
            std::cout << "Device Changed" << std::endl;
            const Leap::DeviceList devices = controller.devices();

            for (int i = 0; i < devices.count(); ++i) {
                std::cout << "id: " << devices[i].toString() << std::endl;
                std::cout << "  isStreaming: " << (devices[i].isStreaming() ? "true" : "false") << std::endl;
            }
        }

        void onServiceConnect(const Leap::Controller& controller) {
            std::cout << "Service Connected" << std::endl;
        }

        void onServiceDisconnect(const Leap::Controller& controller) {
            std::cout << "Service Disconnected" << std::endl;
        }

        void onFocusGained(const Leap::Controller& controller) {
            std::cout << "Focus Gained" << std::endl;
        }

        void onFocusLost(const Leap::Controller& controller) {
            std::cout << "Focus Lost" << std::endl;
        }
    };
}}

cpp_class!(pub unsafe struct LeapFrame as "Leap::Frame");

cpp_class!(pub unsafe struct Listener as "SampleListener");

impl Listener {
    pub fn create() -> &'static Listener {
        cpp!(unsafe [] -> &Listener as "SampleListener*" {
            return new SampleListener();
        })
    }

    fn on_init() {
        println!("onInit")
    }
}

cpp_class!(pub unsafe struct Controller as "Leap::Controller");
impl Controller {
    pub fn create() -> &'static Controller {
        cpp!(unsafe [] -> &Controller as "Leap::Controller*" {
            return new Leap::Controller();
        })
    }

    pub fn add_listener(&self, listener: &Listener) {
        cpp!(unsafe [self as "Leap::Controller*", listener as "Leap::Listener*"] {
            self->addListener(*listener);
        })
    }
}

fn main() {
    let controller = Controller::create();
    let listener = Listener::create();
    controller.add_listener(&listener);
    thread::sleep(Duration::from_secs(2));
}
