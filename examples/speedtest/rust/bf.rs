use crate::common::{mix, MIX_SEED};

const TAPE_SIZE: usize = 30_000;
const PROG_MAX: usize = 64;
const BF_PROGRAM: &[u8] = b"-[>-[>-[>+<-]<-]<-]";

pub fn bf() -> u64 {
    let mut tape = vec![0u8; TAPE_SIZE];
    let mut jump = [0usize; PROG_MAX];
    let mut stack = [0usize; PROG_MAX];
    let mut sp = 0;
    let len = BF_PROGRAM.len();
    for pc in 0..len {
        if BF_PROGRAM[pc] == b'[' {
            stack[sp] = pc;
            sp += 1;
        }
        if BF_PROGRAM[pc] != b']' {
            continue;
        }
        sp -= 1;
        let open = stack[sp];
        jump[open] = pc;
        jump[pc] = open;
    }
    let mut steps: u64 = 0;
    let mut pc = 0;
    let mut ptr = 0;
    while pc < len {
        match BF_PROGRAM[pc] {
            b'+' => tape[ptr] = tape[ptr].wrapping_add(1),
            b'-' => tape[ptr] = tape[ptr].wrapping_sub(1),
            b'>' => ptr += 1,
            b'<' => ptr -= 1,
            b'[' => {
                if tape[ptr] == 0 {
                    pc = jump[pc];
                }
            }
            b']' => {
                if tape[ptr] != 0 {
                    pc = jump[pc];
                }
            }
            _ => {}
        }
        pc += 1;
        steps += 1;
    }
    mix(mix(MIX_SEED, steps), tape[3] as u64)
}
