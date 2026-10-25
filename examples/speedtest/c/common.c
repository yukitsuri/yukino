#define _POSIX_C_SOURCE 200809L
#include <time.h>
#include <unistd.h>
#include "common.h"

enum { maxDigits = 20, exitWriteFailed = 2, reportBytes = 64 };

static const u64 nsPerSec = 1000000000ULL;
static const u64 decimalBase = 10;

u64 nowNs(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (u64)ts.tv_sec * nsPerSec + (u64)ts.tv_nsec;
}

static void writeFd(int fd, const u8 *buf, size_t len) {
    size_t done = 0;
    while (done < len) {
        ssize_t n = write(fd, buf + done, len - done);
        if (n <= 0) _exit(exitWriteFailed);
        done += (size_t)n;
    }
}

void writeOut(const u8 *buf, size_t len) {
    writeFd(STDOUT_FILENO, buf, len);
}

size_t fmtU64(u64 v, u8 *out) {
    u8 tmp[maxDigits];
    size_t n = 0;
    do {
        tmp[n++] = (u8)('0' + v % decimalBase);
        v /= decimalBase;
    } while (v != 0);
    for (size_t i = 0; i < n; i++) out[i] = tmp[n - 1 - i];
    return n;
}

void report(u64 checksum, u64 ns) {
    u8 line[reportBytes];
    size_t n = 0;
    line[n++] = 'R';
    line[n++] = ' ';
    n += fmtU64(checksum, line + n);
    line[n++] = ' ';
    n += fmtU64(ns, line + n);
    line[n++] = '\n';
    writeFd(STDERR_FILENO, line, n);
}
