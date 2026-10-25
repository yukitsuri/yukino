#include "common.h"
#include "tests.h"

enum { exitUsage = 64, exitUnknown = 65 };

typedef struct {
    const char *name;
    u64 (*run)(void);
} Test;

static u64 testNop(void) {
    return 0;
}

static const Test tests[] = {
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

static int strEq(const char *a, const char *b) {
    while (*a != '\0' && *a == *b) {
        a++;
        b++;
    }
    return *a == *b;
}

int main(int argc, char **argv) {
    if (argc != 2) return exitUsage;
    for (size_t i = 0; i < sizeof tests / sizeof tests[0]; i++) {
        if (!strEq(argv[1], tests[i].name)) continue;
        u64 start = nowNs();
        u64 sum = tests[i].run();
        report(sum, nowNs() - start);
        return 0;
    }
    return exitUnknown;
}
