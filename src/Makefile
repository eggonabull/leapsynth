OS := $(shell uname)
ARCH := $(shell uname -m)

LEAP_BASE := /home/drew/Downloads/LeapDeveloperKit_2.3.1+31549_linux/LeapSDK

ifeq ($(OS), Linux)
  ifeq ($(ARCH), x86_64)
	LEAP_ARCH := $(LEAP_BASE)/lib/x64
  else
	LEAP_ARCH := $(LEAP_BASE)/lib/x86
  endif
  LEAP_LIBRARY := $(LEAP_ARCH)/libLeap.so -Wl,-rpath,$(LEAP_ARCH)
else
  # OS X
  LEAP_LIBRARY := $(LEAP_BASE)/lib/libLeap.dylib
endif

all: LeapRust.so

# Convert library code to Object file
libLeapRust.o: LeapRust.cpp
	find .. -name bindings.rs -exec rm {} \;
	mkdir -p ../target/debug/build/
	$(CXX) -Wall -fpic -c -g -I $(LEAP_BASE)/include -o ../target/debug/build/libLeapRust.o LeapRust.cpp $(LEAP_LIBRARY) -D"USE_EXTERN=USE_EXTERN"

# Create shared .SO library
LeapRust.so: libLeapRust.o
	$(CXX) -g -shared -o $(LEAP_ARCH)/libLeapRust.so ../target/debug/build/libLeapRust.o $(LEAP_LIBRARY)

.PHONY: clean

clean:
	rm -f ../target/debug/build/libLeapRust.o /home/drew/Downloads/LeapDeveloperKit_2.3.1+31549_linux/LeapSDK/lib/x64/libLeapRust.so