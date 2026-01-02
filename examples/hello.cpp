#include <iostream>
#include <string>

class Greeter {
private:
    std::string name;

public:
    Greeter(const std::string& n) : name(n) {}
    
    void greet() {
        std::cout << "Hello, " << name << "!" << std::endl;
    }
};

int main() {
    std::cout << "Hello, World!" << std::endl;
    
    Greeter greeter("C++");
    greeter.greet();
    
    return 0;
}

int add(int a, int b) {
    return a + b;
}

namespace Math {
    int multiply(int a, int b) {
        return a * b;
    }
}