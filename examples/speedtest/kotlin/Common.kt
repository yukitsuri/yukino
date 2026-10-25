import java.io.FileDescriptor
import java.io.FileOutputStream
import java.io.IOException
import kotlin.system.exitProcess

const val MIX_SEED: Long = -0x340d631b7bdddcdbL
const val RNG_SEED: Long = -0x61c8864680b583ebL
private const val MIX_PRIME: Long = 0x100000001b3L
private const val RNG_MUL: Long = 0x2545f4914f6cdd1dL
private const val RNG_SHIFT_A = 12
private const val RNG_SHIFT_B = 25
private const val RNG_SHIFT_C = 27
private const val KEY31_SHIFT = 33
private const val FILL_MUL = 167
private const val FILL_SHIFT = 9
private const val DECIMAL_BASE = 10L
private const val MAX_DIGITS = 20
private const val REPORT_BYTES = 64
private const val EXIT_WRITE_FAILED = 2

private val stdout = FileOutputStream(FileDescriptor.out)
private val stderr = FileOutputStream(FileDescriptor.err)

class Rng(seed: Long) {
    private var state: Long = seed

    fun next(): Long {
        var x = state
        x = x xor (x ushr RNG_SHIFT_A)
        x = x xor (x shl RNG_SHIFT_B)
        x = x xor (x ushr RNG_SHIFT_C)
        state = x
        return x * RNG_MUL
    }

    fun key31(): Int = (next() ushr KEY31_SHIFT).toInt()

    fun below(n: Long): Long = (next() ushr 1) % n
}

fun mix(h: Long, v: Long): Long = (h xor v) * MIX_PRIME

fun fillByte(i: Int): Byte = (i * FILL_MUL + (i ushr FILL_SHIFT)).toByte()

fun writeOut(buf: ByteArray, len: Int) {
    try {
        stdout.write(buf, 0, len)
    } catch (e: IOException) {
        exitProcess(EXIT_WRITE_FAILED)
    }
}

fun fmtU64(value: Long, out: ByteArray, off: Int): Int {
    var v = value
    val tmp = ByteArray(MAX_DIGITS)
    var n = 0
    do {
        // long is signed, so halve first to divide values above 2^63
        val q = if (v >= 0) v / DECIMAL_BASE else (v ushr 1) / 5
        tmp[n++] = ('0'.code + (v - q * DECIMAL_BASE).toInt()).toByte()
        v = q
    } while (v != 0L)
    for (i in 0 until n) out[off + i] = tmp[n - 1 - i]
    return n
}

fun report(checksum: Long, ns: Long) {
    val line = ByteArray(REPORT_BYTES)
    line[0] = 'R'.code.toByte()
    line[1] = ' '.code.toByte()
    var n = 2
    n += fmtU64(checksum, line, n)
    line[n++] = ' '.code.toByte()
    n += fmtU64(ns, line, n)
    line[n++] = '\n'.code.toByte()
    try {
        stderr.write(line, 0, n)
    } catch (e: IOException) {
        exitProcess(EXIT_WRITE_FAILED)
    }
}
