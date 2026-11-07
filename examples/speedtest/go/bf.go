package main

const (
	tapeSize  = 30000
	progMax   = 64
	bfProgram = "-[>-[>-[>+<-]<-]<-]"
)

func testBf() uint64 {
	tape := make([]byte, tapeSize)
	var jump, stack [progMax]int
	sp := 0
	length := len(bfProgram)
	for pc := 0; pc < length; pc++ {
		if bfProgram[pc] == '[' {
			stack[sp] = pc
			sp++
		}
		if bfProgram[pc] != ']' {
			continue
		}
		sp--
		open := stack[sp]
		jump[open] = pc
		jump[pc] = open
	}
	var steps uint64
	pc := 0
	ptr := 0
	for pc < length {
		switch bfProgram[pc] {
		case '+':
			tape[ptr]++
		case '-':
			tape[ptr]--
		case '>':
			ptr++
		case '<':
			ptr--
		case '[':
			if tape[ptr] == 0 {
				pc = jump[pc]
			}
		case ']':
			if tape[ptr] != 0 {
				pc = jump[pc]
			}
		}
		pc++
		steps++
	}
	return mix(mix(mixSeed, steps), uint64(tape[3]))
}
