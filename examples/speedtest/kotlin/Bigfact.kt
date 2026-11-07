private const val FACT_N = 20000L
private const val LIMB_CAP = 10000
private const val LIMB_BASE = 1000000000L

fun testBigfact(): Long {
    val limbs = IntArray(LIMB_CAP)
    var used = 1
    limbs[0] = 1
    var m = 2L
    while (m <= FACT_N) {
        var carry = 0L
        for (i in 0 until used) {
            val t = limbs[i].toLong() * m + carry
            limbs[i] = (t % LIMB_BASE).toInt()
            carry = t / LIMB_BASE
        }
        while (carry != 0L) {
            limbs[used++] = (carry % LIMB_BASE).toInt()
            carry /= LIMB_BASE
        }
        m++
    }
    var h = MIX_SEED
    for (i in 0 until used) h = mix(h, limbs[i].toLong())
    return mix(h, used.toLong())
}
