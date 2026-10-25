#include "tests.hpp"

namespace {

constexpr u64 factN = 20000;
constexpr std::size_t limbCap = 10000;
constexpr u64 limbBase = 1000000000ULL;

u32 limbs[limbCap];

}

u64 testBigfact() {
    std::size_t used = 1;
    limbs[0] = 1;
    for (u64 m = 2; m <= factN; m++) {
        u64 carry = 0;
        for (std::size_t i = 0; i < used; i++) {
            u64 t = u64{limbs[i]} * m + carry;
            limbs[i] = static_cast<u32>(t % limbBase);
            carry = t / limbBase;
        }
        while (carry != 0) {
            limbs[used++] = static_cast<u32>(carry % limbBase);
            carry /= limbBase;
        }
    }
    u64 h = mixSeed;
    for (std::size_t i = 0; i < used; i++) h = mix(h, limbs[i]);
    return mix(h, used);
}
