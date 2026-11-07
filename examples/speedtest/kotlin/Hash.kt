private const val HASH_BUF_BYTES = 1 shl 22
private const val SHA_PASSES = 16
private const val CRC_PASSES = 32
private const val FNV_PASSES = 64
private const val SHA_BLOCK_BYTES = 64
private const val SHA_ROUNDS = 64
private const val CRC_POLY: Int = 0xedb88320L.toInt()
private const val U32_MASK = 0xFFFFFFFFL
private const val BYTE_MASK = 0xFF

private val SHA_INIT_HEX = longArrayOf(
    0x6a09e667L, 0xbb67ae85L, 0x3c6ef372L, 0xa54ff53aL, 0x510e527fL, 0x9b05688cL, 0x1f83d9abL, 0x5be0cd19L,
)

private val SHA_K_HEX = longArrayOf(
    0x428a2f98L, 0x71374491L, 0xb5c0fbcfL, 0xe9b5dba5L, 0x3956c25bL, 0x59f111f1L, 0x923f82a4L, 0xab1c5ed5L,
    0xd807aa98L, 0x12835b01L, 0x243185beL, 0x550c7dc3L, 0x72be5d74L, 0x80deb1feL, 0x9bdc06a7L, 0xc19bf174L,
    0xe49b69c1L, 0xefbe4786L, 0x0fc19dc6L, 0x240ca1ccL, 0x2de92c6fL, 0x4a7484aaL, 0x5cb0a9dcL, 0x76f988daL,
    0x983e5152L, 0xa831c66dL, 0xb00327c8L, 0xbf597fc7L, 0xc6e00bf3L, 0xd5a79147L, 0x06ca6351L, 0x14292967L,
    0x27b70a85L, 0x2e1b2138L, 0x4d2c6dfcL, 0x53380d13L, 0x650a7354L, 0x766a0abbL, 0x81c2c92eL, 0x92722c85L,
    0xa2bfe8a1L, 0xa81a664bL, 0xc24b8b70L, 0xc76c51a3L, 0xd192e819L, 0xd6990624L, 0xf40e3585L, 0x106aa070L,
    0x19a4c116L, 0x1e376c08L, 0x2748774cL, 0x34b0bcb5L, 0x391c0cb3L, 0x4ed8aa4aL, 0x5b9cca4fL, 0x682e6ff3L,
    0x748f82eeL, 0x78a5636fL, 0x84c87814L, 0x8cc70208L, 0x90befffaL, 0xa4506cebL, 0xbef9a3f7L, 0xc67178f2L,
)

private val SHA_K = IntArray(SHA_ROUNDS) { SHA_K_HEX[it].toInt() }

private fun makeBuf(): ByteArray {
    val buf = ByteArray(HASH_BUF_BYTES)
    for (i in 0 until HASH_BUF_BYTES) buf[i] = fillByte(i)
    return buf
}

private fun ror(x: Int, n: Int): Int = (x ushr n) or (x shl (32 - n))

private fun shaBlock(st: IntArray, p: ByteArray, off: Int, w: IntArray) {
    for (i in 0 until 16) {
        val o = off + 4 * i
        w[i] = ((p[o].toInt() and BYTE_MASK) shl 24) or ((p[o + 1].toInt() and BYTE_MASK) shl 16) or
            ((p[o + 2].toInt() and BYTE_MASK) shl 8) or (p[o + 3].toInt() and BYTE_MASK)
    }
    for (i in 16 until SHA_ROUNDS) {
        val s0 = ror(w[i - 15], 7) xor ror(w[i - 15], 18) xor (w[i - 15] ushr 3)
        val s1 = ror(w[i - 2], 17) xor ror(w[i - 2], 19) xor (w[i - 2] ushr 10)
        w[i] = w[i - 16] + s0 + w[i - 7] + s1
    }
    var a = st[0]
    var b = st[1]
    var c = st[2]
    var d = st[3]
    var e = st[4]
    var f = st[5]
    var g = st[6]
    var h = st[7]
    for (i in 0 until SHA_ROUNDS) {
        val t1 = h + (ror(e, 6) xor ror(e, 11) xor ror(e, 25)) + ((e and f) xor (e.inv() and g)) + SHA_K[i] + w[i]
        val t2 = (ror(a, 2) xor ror(a, 13) xor ror(a, 22)) + ((a and b) xor (a and c) xor (b and c))
        h = g
        g = f
        f = e
        e = d + t1
        d = c
        c = b
        b = a
        a = t1 + t2
    }
    st[0] += a
    st[1] += b
    st[2] += c
    st[3] += d
    st[4] += e
    st[5] += f
    st[6] += g
    st[7] += h
}

fun testSha256(): Long {
    val buf = makeBuf()
    val st = IntArray(8) { SHA_INIT_HEX[it].toInt() }
    val w = IntArray(SHA_ROUNDS)
    for (pass in 0 until SHA_PASSES) {
        var off = 0
        while (off < HASH_BUF_BYTES) {
            shaBlock(st, buf, off, w)
            off += SHA_BLOCK_BYTES
        }
    }
    val last = ByteArray(SHA_BLOCK_BYTES)
    val bits = SHA_PASSES.toLong() * HASH_BUF_BYTES * 8
    last[0] = 0x80.toByte()
    for (k in 0 until 8) last[SHA_BLOCK_BYTES - 1 - k] = (bits ushr (8 * k)).toByte()
    shaBlock(st, last, 0, w)
    var h = MIX_SEED
    for (i in 0 until 8) h = mix(h, st[i].toLong() and U32_MASK)
    return h
}

fun testCrc32(): Long {
    val buf = makeBuf()
    val table = IntArray(256)
    for (n in 0 until 256) {
        var c = n
        for (k in 0 until 8) c = if ((c and 1) == 1) (c ushr 1) xor CRC_POLY else c ushr 1
        table[n] = c
    }
    var crc = -1
    for (pass in 0 until CRC_PASSES) {
        for (i in 0 until HASH_BUF_BYTES) {
            crc = table[(crc xor (buf[i].toInt() and BYTE_MASK)) and BYTE_MASK] xor (crc ushr 8)
        }
    }
    return mix(MIX_SEED, crc.inv().toLong() and U32_MASK)
}

fun testFnv(): Long {
    val buf = makeBuf()
    var h = MIX_SEED
    for (pass in 0 until FNV_PASSES) {
        for (i in 0 until HASH_BUF_BYTES) h = mix(h, (buf[i].toInt() and BYTE_MASK).toLong())
    }
    return h
}
