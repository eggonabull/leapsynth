#include "LeapRust.h"
#include "Leap.h"
#include <stdio.h>

LeapRustVector convert_vector(Leap::Vector vect) {
    return LeapRustVector {
        x: vect.x,
        y: vect.y,
        z: vect.z
    };
}

LeapRustMatrix convert_matrix(Leap::Matrix matrix) {
    return LeapRustMatrix {
        xBasis: convert_vector(matrix.xBasis),
        yBasis: convert_vector(matrix.yBasis),
        zBasis: convert_vector(matrix.zBasis),
        origin: convert_vector(matrix.origin)
    };
}

class SampleListener : public Leap::Listener {
  public:
    SampleListener(struct LeapRustController* lrcontroller) {
        this->lrcontroller = lrcontroller;
    }
    virtual void onInit(const Leap::Controller&);
    virtual void onConnect(const Leap::Controller&);
    virtual void onDisconnect(const Leap::Controller&);
    virtual void onExit(const Leap::Controller&);
    virtual void onFrame(const Leap::Controller&);
    virtual void onFocusGained(const Leap::Controller&);
    virtual void onFocusLost(const Leap::Controller&);
    virtual void onDeviceChange(const Leap::Controller&);
    virtual void onServiceConnect(const Leap::Controller&);
    virtual void onServiceDisconnect(const Leap::Controller&);

  private:
    struct LeapRustController* lrcontroller;
};

void SampleListener::onInit(const Leap::Controller& controller) {
    std::cout << "Initialized" << std::endl;
}

void SampleListener::onConnect(const Leap::Controller& controller) {
    std::cout << "Connected" << std::endl;
    //   controller.enableGesture(Gesture::TYPE_CIRCLE);
    //   controller.enableGesture(Gesture::TYPE_KEY_TAP);
    //   controller.enableGesture(Gesture::TYPE_SCREEN_TAP);
    //   controller.enableGesture(Gesture::TYPE_SWIPE);
}

void SampleListener::onDisconnect(const Leap::Controller& controller) {
  // Note: not dispatched when running in a debugger.
  std::cout << "Disconnected" << std::endl;
}

void SampleListener::onExit(const Leap::Controller& controller) {
  std::cout << "Exited" << std::endl;
}

void SampleListener::onFrame(const Leap::Controller& controller) {
  // Get the most recent frame and report some basic information
  LeapRustFrame frame;
  get_frame_from_controller(this->lrcontroller, &frame);
  this->lrcontroller->on_frame_callback(this->lrcontroller->env, &frame);
}

void SampleListener::onDeviceChange(const Leap::Controller& controller) {
  std::cout << "Device Changed" << std::endl;
  const Leap::DeviceList devices = controller.devices();

  for (int i = 0; i < devices.count(); ++i) {
    std::cout << "id: " << devices[i].toString() << std::endl;
    std::cout << "  isStreaming: " << (devices[i].isStreaming() ? "true" : "false") << std::endl;
  }
}

void SampleListener::onServiceConnect(const Leap::Controller& controller) {
  std::cout << "Service Connected" << std::endl;
}

void SampleListener::onServiceDisconnect(const Leap::Controller& controller) {
  std::cout << "Service Disconnected" << std::endl;
}


void SampleListener::onFocusGained(const Leap::Controller& controller) {
  std::cout << "Focus Gained" << std::endl;
}

void SampleListener::onFocusLost(const Leap::Controller& controller) {
  std::cout << "Focus Lost" << std::endl;
}


extern "C" {

struct LeapRustFrame* blank_frame() {
    return (LeapRustFrame*)calloc(1, (sizeof(LeapRustFrame)));
}

struct LeapRustController* get_controller(struct LeapRustEnv* env, FrameCallback on_frame_callback) {
    LeapRustController* controller_struct = (LeapRustController *)malloc(sizeof(LeapRustController));
    Leap::Controller* controller = new Leap::Controller();
    controller_struct->controller = (void *)controller;
    controller_struct->on_frame_callback = on_frame_callback;
    controller_struct->env = env;
    return controller_struct;
}

void add_listener(struct LeapRustController* lrcontroller) {
    Leap::Controller* controller = (Leap::Controller*)lrcontroller->controller;
    SampleListener* sampleListener = new SampleListener(lrcontroller);
    controller->addListener(*sampleListener);
}

void get_frame_from_controller(struct LeapRustController* lrcontroller, struct LeapRustFrame *const rustFrame) {
    Leap::Controller* controller = (Leap::Controller*)lrcontroller->controller;
    const Leap::Frame leapFrame = controller->frame();
    rustFrame->id = leapFrame.id();
    rustFrame->timestamp = leapFrame.timestamp();
    rustFrame->handCount = leapFrame.hands().count() > 2 ? 2 : leapFrame.hands().count();

    // std::cout << "Frame id: " << leapFrame.id()
    //         << ", timestamp: " << leapFrame.timestamp()
    //         << ", hands: " << leapFrame.hands().count()
    //         << ", extended fingers: " << leapFrame.fingers().extended().count()
    //         << ", tools: " << leapFrame.tools().count()
    //         << ", gestures: " << leapFrame.gestures().count() << std::endl;

    const Leap::HandList hands = leapFrame.hands();
    int hands_recorded = 0;

    for (Leap::HandList::const_iterator hl = hands.begin(); hl != hands.end(); ++hl, ++hands_recorded) {
        const Leap::Hand hand = *hl;
        LeapRustHand *lrHand = &(rustFrame->hands[hands_recorded]);
        lrHand->fingerCount = hand.fingers().count() > 5 ? 5 : hand.fingers().count();
        lrHand->isLeft = hand.isLeft();

        // std::string handType = hand.isLeft() ? "Left hand" : "Right hand";
        // std::cout << std::string(2, ' ') << handType << ", id: " << hand.id()
        //       << ", palm position: " << hand.palmPosition() << std::endl;

        /* record arm stuff */
        Leap::Arm arm = hand.arm();
        lrHand->arm.width = arm.width();
        lrHand->arm.basis = convert_matrix(arm.basis());
        lrHand->arm.direction = convert_vector(arm.direction());
        lrHand->arm.wristPosition = convert_vector(arm.wristPosition());
        lrHand->arm.center = convert_vector(arm.center());
        lrHand->arm.elbowPosition = convert_vector(arm.elbowPosition());

        /* record hand stuff */
        lrHand->id = hand.id();
        lrHand->palmPosition = convert_vector(hand.palmPosition());
        lrHand->stabilizedPalmPosition = convert_vector(hand.palmPosition());
        lrHand->palmWidth = hand.palmWidth();
        lrHand->palmVelocity = convert_vector(hand.palmVelocity());
        lrHand->palmNormal = convert_vector(hand.palmNormal());
        lrHand->direction = convert_vector(hand.direction());
        lrHand->basis = convert_matrix(hand.basis());
        lrHand->wristPosition = convert_vector(hand.wristPosition());
        lrHand->sphereCenter = convert_vector(hand.sphereCenter());
        lrHand->sphereRadius = hand.sphereRadius();
        lrHand->pinchStrength = hand.pinchStrength();
        lrHand->grabStrength = hand.grabStrength();


        int fingers_recorded = 0;
        Leap::FingerList fingers = hand.fingers();

        for (Leap::FingerList::const_iterator fl = fingers.begin(); fl != fingers.end(), fingers_recorded < 5; ++fl, ++fingers_recorded) {
            const Leap::Finger finger = *fl;
            LeapRustFinger *lrFinger = &(lrHand->fingers[fingers_recorded]);

            /* record finger stuff */
            lrFinger->tipPosition = convert_vector(finger.tipPosition());
            lrFinger->tipVelocity = convert_vector(finger.tipVelocity());
            lrFinger->id = finger.id();
            lrFinger->length = finger.length();
            lrFinger->width = finger.width();

            /* record finger bones */
            lrFinger->boneCount = 4;
            for (int bones_recorded = 0; bones_recorded < 4; ++bones_recorded) {
                Leap::Bone bone = finger.bone((Leap::Bone::Type)bones_recorded);
                LeapRustBone *lrBone = &(lrFinger->bones[bones_recorded]);

                /* record bone stuff */
                lrBone->basis = convert_matrix(bone.basis());
                lrBone->prevJoint = convert_vector(bone.prevJoint());
                lrBone->nextJoint = convert_vector(bone.nextJoint());
                lrBone->center = convert_vector(bone.center());
                lrBone->direction = convert_vector(bone.direction());
            }
        }
        // const Vector normal = hand.palmNormal();
        // const Vector direction = hand.direction();
        // std::cout << std::string(2, ' ') <<  "pitch: " << direction.pitch() * RAD_TO_DEG << " degrees, "
        //       << "roll: " << normal.roll() * RAD_TO_DEG << " degrees, "
        //       << "yaw: " << direction.yaw() * RAD_TO_DEG << " degrees" << std::endl;
    }
}

}