private const val CHACHA_BLOCKS = 2000000
private const val CHACHA_WORDS = 16
private const val CHACHA_DOUBLE_ROUNDS = 10
private const val U32_MASK = 0xFFFFFFFFL
private val CHACHA_CONST = intArrayOf(0x61707865, 0x3320646e, 0x79622d32, 0x6b206574)

private fun rol(x: Int, n: Int): Int = (x shl n) or (x ushr (32 - n))

private fun quarter(s: IntArray, a: Int, b: Int, c: Int, d: Int) {
    s[a] += s[b]
    s[d] = rol(s[d] xor s[a], 16)
    s[c] += s[d]
    s[b] = rol(s[b] xor s[c], 12)
    s[a] += s[b]
    s[d] = rol(s[d] xor s[a], 8)
    s[c] += s[d]
    s[b] = rol(s[b] xor s[c], 7)
}

private fun chachaBlock(input: IntArray, out: IntArray) {
    for (i in 0 until CHACHA_WORDS) out[i] = input[i]
    for (r in 0 until CHACHA_DOUBLE_ROUNDS) {
        quarter(out, 0, 4, 8, 12)
        quarter(out, 1, 5, 9, 13)
        quarter(out, 2, 6, 10, 14)
        quarter(out, 3, 7, 11, 15)
        quarter(out, 0, 5, 10, 15)
        quarter(out, 1, 6, 11, 12)
        quarter(out, 2, 7, 8, 13)
        quarter(out, 3, 4, 9, 14)
    }
    for (i in 0 until CHACHA_WORDS) out[i] += input[i]
}

fun testChacha(): Long {
    val st = IntArray(CHACHA_WORDS)
    val out = IntArray(CHACHA_WORDS)
    val r = Rng(RNG_SEED)
    for (i in 0 until 4) st[i] = CHACHA_CONST[i]
    for (i in 4 until CHACHA_WORDS) st[i] = r.next().toInt()
    var acc = 0
    for (c in 0 until CHACHA_BLOCKS) {
        st[12] = c
        chachaBlock(st, out)
        for (i in 0 until CHACHA_WORDS) acc = acc xor out[i]
    }
    return mix(MIX_SEED, acc.toLong() and U32_MASK)
}
