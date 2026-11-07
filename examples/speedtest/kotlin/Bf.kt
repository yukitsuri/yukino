private const val TAPE_SIZE = 30000
private const val PROG_MAX = 64
private const val BF_PROGRAM = "-[>-[>-[>+<-]<-]<-]"

fun testBf(): Long {
    val tape = ByteArray(TAPE_SIZE)
    val jump = IntArray(PROG_MAX)
    val stack = IntArray(PROG_MAX)
    var sp = 0
    val len = BF_PROGRAM.length
    for (pc in 0 until len) {
        if (BF_PROGRAM[pc] == '[') stack[sp++] = pc
        if (BF_PROGRAM[pc] != ']') continue
        val open = stack[--sp]
        jump[open] = pc
        jump[pc] = open
    }
    var steps = 0L
    var pc = 0
    var ptr = 0
    while (pc < len) {
        when (BF_PROGRAM[pc]) {
            '+' -> tape[ptr]++
            '-' -> tape[ptr]--
            '>' -> ptr++
            '<' -> ptr--
            '[' -> if (tape[ptr].toInt() == 0) pc = jump[pc]
            ']' -> if (tape[ptr].toInt() != 0) pc = jump[pc]
        }
        pc++
        steps++
    }
    return mix(mix(MIX_SEED, steps), (tape[3].toInt() and 0xFF).toLong())
}
