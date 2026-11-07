private const val SIEVE_LIMIT = 40000000
private const val COLLATZ_LIMIT = 1500000L
private const val FIB_N = 42L
private const val QUEENS_N = 14
private const val LEIBNIZ_TERMS = 150000000L
private const val LEIBNIZ_SCALE = 4.0e9

fun testSieve(): Long {
    val composite = ByteArray(SIEVE_LIMIT + 1)
    var i = 2
    while (i.toLong() * i <= SIEVE_LIMIT) {
        if (composite[i].toInt() == 0) {
            var j = i * i
            while (j <= SIEVE_LIMIT) {
                composite[j] = 1
                j += i
            }
        }
        i++
    }
    var count = 0L
    for (k in 2..SIEVE_LIMIT) {
        if (composite[k].toInt() == 0) count++
    }
    return mix(MIX_SEED, count)
}

fun testCollatz(): Long {
    var bestStart = 0L
    var bestSteps = 0L
    var start = 1L
    while (start <= COLLATZ_LIMIT) {
        var n = start
        var steps = 0L
        while (n != 1L) {
            n = if ((n and 1L) == 1L) 3 * n + 1 else n ushr 1
            steps++
        }
        if (steps > bestSteps) {
            bestSteps = steps
            bestStart = start
        }
        start++
    }
    return mix(mix(MIX_SEED, bestStart), bestSteps)
}

private fun fib(n: Long): Long = if (n < 2) n else fib(n - 1) + fib(n - 2)

fun testFib(): Long = mix(MIX_SEED, fib(FIB_N))

private fun solveQueens(cols: Int, diagL: Int, diagR: Int, full: Int): Long {
    if (cols == full) return 1L
    var count = 0L
    var avail = full and (cols or diagL or diagR).inv()
    while (avail != 0) {
        val bit = avail and (avail.inv() + 1)
        avail = avail xor bit
        count += solveQueens(cols or bit, ((diagL or bit) shl 1) and full, (diagR or bit) ushr 1, full)
    }
    return count
}

fun testQueens(): Long {
    val full = (1 shl QUEENS_N) - 1
    return mix(MIX_SEED, solveQueens(0, 0, 0, full))
}

fun testLeibniz(): Long {
    var sum = 0.0
    var sign = 1.0
    var k = 0L
    while (k < LEIBNIZ_TERMS) {
        sum += sign / (2 * k + 1).toDouble()
        sign = -sign
        k++
    }
    return mix(MIX_SEED, (sum * LEIBNIZ_SCALE).toLong())
}
