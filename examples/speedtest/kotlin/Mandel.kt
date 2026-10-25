private const val MANDEL_SIZE = 1000L
private const val MANDEL_MAX_ITER = 500L
private const val MANDEL_FRAC = 24

private fun iterations(cr: Long, ci: Long): Long {
    val limit = 4L shl MANDEL_FRAC
    var zr = 0L
    var zi = 0L
    var n = 0L
    while (n < MANDEL_MAX_ITER) {
        val zr2 = (zr * zr) shr MANDEL_FRAC
        val zi2 = (zi * zi) shr MANDEL_FRAC
        if (zr2 + zi2 > limit) break
        zi = ((2 * zr * zi) shr MANDEL_FRAC) + ci
        zr = zr2 - zi2 + cr
        n++
    }
    return n
}

fun testMandel(): Long {
    val one = 1L shl MANDEL_FRAC
    val x0 = -2 * one
    val y0 = -3 * one / 2
    val step = 3 * one / MANDEL_SIZE
    var total = 0L
    var py = 0L
    while (py < MANDEL_SIZE) {
        var px = 0L
        while (px < MANDEL_SIZE) {
            total += iterations(x0 + px * step, y0 + py * step)
            px++
        }
        py++
    }
    return mix(MIX_SEED, total)
}
