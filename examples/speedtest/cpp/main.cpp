#include "common.hpp"
#include "tests.hpp"

namespace {

constexpr int exitUsage = 64;
constexpr int exitUnknown = 65;

struct Test {
    const char *name;
    u64 (*run)();
};

u64 testNop() {
    return 0;
}

const Test tests[] = {
    {"nop", testNop},
    {"sieve", testSieve},
    {"collatz", testCollatz},
    {"fib", testFib},
    {"queens", testQueens},
    {"leibniz", testLeibniz},
    {"matmul", testMatmul},
    {"spectral", testSpectral},
    {"mandel", testMandel},
    {"sort", testSort},
    {"hashmap", testHashmap},
    {"sha256", testSha256},
    {"crc32", testCrc32},
    {"fnv", testFnv},
    {"chacha", testChacha},
    {"bigfact", testBigfact},
    {"bf", testBf},
    {"memseq", testMemSeq},
    {"memrand", testMemRand},
    {"memtree", testMemTree},
    {"outbuf", testOutBuf},
    {"outline", testOutLine},
};

bool strEq(const char *a, const char *b) {
    while (*a != '\0' && *a == *b) {
        a++;
        b++;
    }
    return *a == *b;
}

}

int main(int argc, char **argv) {
    if (argc != 2) return exitUsage;
    for (const Test &t : tests) {
        if (!strEq(argv[1], t.name)) continue;
        u64 start = nowNs();
        u64 sum = t.run();
        report(sum, nowNs() - start);
        return 0;
    }
    return exitUnknown;
}
