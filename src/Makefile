OS := $(shell uname)
ARCH := $(shell uname -m)

ifndef LEAP_BASE
$(error LEAP_BASE is not set)
endif


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

all: $(LEAP_ARCH)/libLeapRust.so

# Convert library code to Object file
../target/debug/build/libLeapRust.o: LeapRust.cpp
	find .. -name bindings.rs -exec rm {} \;
	mkdir -p ../target/debug/build/
	$(CXX) -Wall -fpic -c -g -I $(LEAP_BASE)/include -o ../target/debug/build/libLeapRust.o LeapRust.cpp $(LEAP_LIBRARY) -D"USE_EXTERN=USE_EXTERN"

# Create shared .SO library
$(LEAP_ARCH)/libLeapRust.so: ../target/debug/build/libLeapRust.o
	$(CXX) -g -shared -o $(LEAP_ARCH)/libLeapRust.so ../target/debug/build/libLeapRust.o $(LEAP_LIBRARY)

.PHONY: clean

clean:
	rm -f ../target/debug/build/libLeapRust.o $(LEAP_BASE)/lib/x64/libLeapRust.so