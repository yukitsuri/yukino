#include "tests.hpp"

namespace {

constexpr u32 tapeSize = 30000;
constexpr u32 progMax = 64;
constexpr char bfProgram[] = "-[>-[>-[>+<-]<-]<-]";

}

u64 testBf() {
    u8 tape[tapeSize] = {};
    u32 jump[progMax];
    u32 stack[progMax];
    u32 sp = 0;
    u32 len = 0;
    while (bfProgram[len] != '\0') len++;
    for (u32 pc = 0; pc < len; pc++) {
        if (bfProgram[pc] == '[') stack[sp++] = pc;
        if (bfProgram[pc] != ']') continue;
        u32 open = stack[--sp];
        jump[open] = pc;
        jump[pc] = open;
    }
    u64 steps = 0;
    u32 pc = 0;
    u32 ptr = 0;
    while (pc < len) {
        switch (bfProgram[pc]) {
        case '+': tape[ptr]++; break;
        case '-': tape[ptr]--; break;
        case '>': ptr++; break;
        case '<': ptr--; break;
        case '[': if (tape[ptr] == 0) pc = jump[pc]; break;
        case ']': if (tape[ptr] != 0) pc = jump[pc]; break;
        default: break;
        }
        pc++;
        steps++;
    }
    return mix(mix(mixSeed, steps), tape[3]);
}
