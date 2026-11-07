const common = @import("common.zig");

const tapeSize = 30000;
const progMax = 64;
const bfProgram = "-[>-[>-[>+<-]<-]<-]";

pub fn bf() u64 {
    var tape: [tapeSize]u8 = @splat(0);
    var jump: [progMax]u32 = undefined;
    var stack: [progMax]u32 = undefined;
    var sp: usize = 0;
    const len: u32 = bfProgram.len;
    var pc: u32 = 0;
    while (pc < len) : (pc += 1) {
        if (bfProgram[pc] == '[') {
            stack[sp] = pc;
            sp += 1;
        }
        if (bfProgram[pc] != ']') continue;
        sp -= 1;
        const open = stack[sp];
        jump[open] = pc;
        jump[pc] = open;
    }
    var steps: u64 = 0;
    var ptr: u32 = 0;
    pc = 0;
    while (pc < len) {
        switch (bfProgram[pc]) {
            '+' => tape[ptr] +%= 1,
            '-' => tape[ptr] -%= 1,
            '>' => ptr += 1,
            '<' => ptr -= 1,
            '[' => if (tape[ptr] == 0) {
                pc = jump[pc];
            },
            ']' => if (tape[ptr] != 0) {
                pc = jump[pc];
            },
            else => {},
        }
        pc += 1;
        steps += 1;
    }
    return common.mix(common.mix(common.mixSeed, steps), tape[3]);
}
