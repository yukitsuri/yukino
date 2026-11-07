private const val MAT_N = 900
private const val MAT_CELLS = MAT_N * MAT_N
private const val SPECTRAL_N = 1500
private const val SPECTRAL_ITERS = 10
private const val SPECTRAL_SCALE = 1.0e6
private const val U32_MASK = 0xFFFFFFFFL

private fun multiply(a: IntArray, b: IntArray, c: IntArray) {
    for (i in 0 until MAT_N) {
        for (k in 0 until MAT_N) {
            val aik = a[i * MAT_N + k]
            val bBase = k * MAT_N
            val cBase = i * MAT_N
            for (j in 0 until MAT_N) c[cBase + j] += aik * b[bBase + j]
        }
    }
}

fun testMatmul(): Long {
    val a = IntArray(MAT_CELLS)
    val b = IntArray(MAT_CELLS)
    val c = IntArray(MAT_CELLS)
    val r = Rng(RNG_SEED)
    for (i in 0 until MAT_CELLS) a[i] = r.key31()
    for (i in 0 until MAT_CELLS) b[i] = r.key31()
    multiply(a, b, c)
    var h = MIX_SEED
    for (i in 0 until MAT_CELLS) h = mix(h, c[i].toLong() and U32_MASK)
    return h
}

private fun coef(i: Int, j: Int): Double = 1.0 / ((i + j) * (i + j + 1) / 2 + i + 1).toDouble()

private fun mulAv(v: DoubleArray, out: DoubleArray) {
    for (i in 0 until SPECTRAL_N) {
        var s = 0.0
        for (j in 0 until SPECTRAL_N) s += coef(i, j) * v[j]
        out[i] = s
    }
}

private fun mulAtv(v: DoubleArray, out: DoubleArray) {
    for (i in 0 until SPECTRAL_N) {
        var s = 0.0
        for (j in 0 until SPECTRAL_N) s += coef(j, i) * v[j]
        out[i] = s
    }
}

private fun mulAtAv(v: DoubleArray, out: DoubleArray, tmp: DoubleArray) {
    mulAv(v, tmp)
    mulAtv(tmp, out)
}

fun testSpectral(): Long {
    val u = DoubleArray(SPECTRAL_N) { 1.0 }
    val v = DoubleArray(SPECTRAL_N)
    val t = DoubleArray(SPECTRAL_N)
    for (it in 0 until SPECTRAL_ITERS) {
        mulAtAv(u, v, t)
        mulAtAv(v, u, t)
    }
    var vbv = 0.0
    var vv = 0.0
    for (i in 0 until SPECTRAL_N) {
        vbv += u[i] * v[i]
        vv += v[i] * v[i]
    }
    return mix(mix(MIX_SEED, (vbv * SPECTRAL_SCALE).toLong()), (vv * SPECTRAL_SCALE).toLong())
}
