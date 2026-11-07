#include <time.h>
#include <unistd.h>
#include "common.hpp"

namespace {

constexpr u32 maxDigits = 20;
constexpr int exitWriteFailed = 2;
constexpr u32 reportBytes = 64;
constexpr u64 nsPerSec = 1000000000ULL;
constexpr u64 decimalBase = 10;

void writeFd(int fd, const u8 *buf, std::size_t len) {
    std::size_t done = 0;
    while (done < len) {
        ssize_t n = write(fd, buf + done, len - done);
        if (n <= 0) _exit(exitWriteFailed);
        done += static_cast<std::size_t>(n);
    }
}

}

u64 nowNs() {
    timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return static_cast<u64>(ts.tv_sec) * nsPerSec + static_cast<u64>(ts.tv_nsec);
}

void writeOut(const u8 *buf, std::size_t len) {
    writeFd(STDOUT_FILENO, buf, len);
}

std::size_t fmtU64(u64 v, u8 *out) {
    u8 tmp[maxDigits];
    std::size_t n = 0;
    do {
        tmp[n++] = static_cast<u8>('0' + v % decimalBase);
        v /= decimalBase;
    } while (v != 0);
    for (std::size_t i = 0; i < n; i++) out[i] = tmp[n - 1 - i];
    return n;
}

void report(u64 checksum, u64 ns) {
    u8 line[reportBytes];
    std::size_t n = 0;
    line[n++] = 'R';
    line[n++] = ' ';
    n += fmtU64(checksum, line + n);
    line[n++] = ' ';
    n += fmtU64(ns, line + n);
    line[n++] = '\n';
    writeFd(STDERR_FILENO, line, n);
}
