#include "tests.h"

enum { factN = 20000, limbCap = 10000 };

static const u64 limbBase = 1000000000ULL;
static u32 limbs[limbCap];

u64 testBigfact(void) {
    size_t used = 1;
    limbs[0] = 1;
    for (u64 m = 2; m <= factN; m++) {
        u64 carry = 0;
        for (size_t i = 0; i < used; i++) {
            u64 t = (u64)limbs[i] * m + carry;
            limbs[i] = (u32)(t % limbBase);
            carry = t / limbBase;
        }
        while (carry != 0) {
            limbs[used++] = (u32)(carry % limbBase);
            carry /= limbBase;
        }
    }
    u64 h = mixSeed;
    for (size_t i = 0; i < used; i++) h = mix(h, limbs[i]);
    return mix(h, used);
}
