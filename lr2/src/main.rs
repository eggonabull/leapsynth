use std;
use cpp::{cpp, cpp_class};

cpp!{{
    #include "memory"
    #include "Leap.h"
}}

cpp!{{
    class MyListener : public Leap::Listener {
      public:
        TraitPtr m_trait;
        int computeValue(int x) const override {
            return rust!(MCI_computeValue [m_trait : &MyTrait as "TraitPtr", x : i32 as "int"]
                -> i32 as "int" {
                m_trait.compute_value(x)
            });
        }
    }
 }}


fn main() {
    let r = unsafe {
        cpp!([] -> {
            std::cout << "Hello, ";
        })
    };
    
}