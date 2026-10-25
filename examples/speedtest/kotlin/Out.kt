private const val OUT_BUF_SIZE = 65536
private const val OUT_LINES = 3000000L
private const val OUT_SINGLE_LINES = 300000L
private const val MAX_LINE_BYTES = 64

private fun fmtLine(dst: ByteArray, off: Int, index: Long, value: Long): Int {
    var n = fmtU64(index, dst, off)
    dst[off + n++] = ' '.code.toByte()
    n += fmtU64(value, dst, off + n)
    dst[off + n++] = '\n'.code.toByte()
    return n
}

fun testOutBuf(): Long {
    val r = Rng(RNG_SEED)
    val buf = ByteArray(OUT_BUF_SIZE)
    var used = 0
    var total = 0L
    for (i in 0 until OUT_LINES) {
        if (used + MAX_LINE_BYTES > OUT_BUF_SIZE) {
            writeOut(buf, used)
            total += used
            used = 0
        }
        used += fmtLine(buf, used, i, r.next())
    }
    writeOut(buf, used)
    return mix(MIX_SEED, total + used)
}

fun testOutLine(): Long {
    val r = Rng(RNG_SEED)
    val line = ByteArray(MAX_LINE_BYTES)
    var total = 0L
    for (i in 0 until OUT_SINGLE_LINES) {
        val n = fmtLine(line, 0, i, r.next())
        writeOut(line, n)
        total += n
    }
    return mix(MIX_SEED, total)
}
