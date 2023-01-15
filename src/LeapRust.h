#pragma once

#ifdef USE_EXTERN
extern "C" {
#endif

const char* fingerNames[] = {"Thumb", "Index", "Middle", "Ring", "Pinky"};
const char* boneNames[] = {"Metacarpal", "Proximal", "Middle", "Distal"};
const char* stateNames[] = {"STATE_INVALID", "STATE_START", "STATE_UPDATE", "STATE_END"};

enum LeapRustBoneType {
  TYPE_METACARPAL = 0,   /**< Bone connected to the wrist inside the palm */
  TYPE_PROXIMAL = 1,     /**< Bone connecting to the palm */
  TYPE_INTERMEDIATE = 2, /**< Bone between the tip and the base*/
  TYPE_DISTAL = 3,       /**< Bone at the tip of the finger */
};

struct LeapRustVector {
  float x;
  float y;
  float z;
};

struct LeapRustMatrix {
  struct LeapRustVector xBasis;
  struct LeapRustVector yBasis;
  struct LeapRustVector zBasis;
  struct LeapRustVector origin;
};

struct LeapRustBone {
  struct LeapRustMatrix basis;
  struct LeapRustVector prevJoint;
  struct LeapRustVector center;
  struct LeapRustVector nextJoint;
  struct LeapRustVector direction;
  enum LeapRustBoneType type;
  float length;
  float width;
  int isValid;
};

enum LeapRustFingerType {
  TYPE_THUMB  = 0, /**< The thumb */
  TYPE_INDEX  = 1, /**< The index or fore-finger */
  TYPE_MIDDLE = 2, /**< The middle finger */
  TYPE_RING   = 3, /**< The ring finger */
  TYPE_PINKY  = 4  /**< The pinky or little finger */
};

struct LeapRustFinger {
  enum LeapRustFingerType type;
  struct LeapRustVector tipPosition;
  struct LeapRustVector tipVelocity;
  int id;
  int length;
  int width;
  int boneCount;
  struct LeapRustBone bones[4];
};

struct LeapRustArm {
  float width;
  struct LeapRustMatrix basis;
  struct LeapRustVector direction;
  struct LeapRustVector wristPosition;
  struct LeapRustVector center;
  struct LeapRustVector elbowPosition;
};

struct LeapRustHand {
  struct LeapRustArm arm;
  int id;
  struct LeapRustVector palmPosition;
  struct LeapRustVector stabilizedPalmPosition;
  float palmWidth;
  struct LeapRustVector palmVelocity;
  struct LeapRustVector palmNormal;
  struct LeapRustVector direction;
  struct LeapRustMatrix basis;
  int isLeft;

  struct LeapRustVector wristPosition;
  struct LeapRustVector sphereCenter;
  float sphereRadius;
  float pinchStrength;
  float grabStrength;

  int fingerCount;
  struct LeapRustFinger fingers[5];
};

struct LeapRustFrame {
  int id;
  int timestamp;
  int handCount;
  struct LeapRustHand hands[2];
};

struct LeapRustEnv {
  struct LeapRustFrame* frame;
};

typedef void (*FrameCallback)(struct LeapRustEnv* env, struct LeapRustFrame*);

struct LeapRustController {
  struct LeapRustEnv *env;
  void *controller;
  FrameCallback on_frame_callback;
};

struct LeapRustFrame* blank_frame();

struct LeapRustController* get_controller(struct LeapRustEnv* env, FrameCallback callback);

void add_listener(struct LeapRustController*);

void get_frame_from_controller(struct LeapRustController*, struct LeapRustFrame *const rustFrame);

#ifdef USE_EXTERN
}
#endif
