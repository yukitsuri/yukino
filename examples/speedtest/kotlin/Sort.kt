private const val SORT_COUNT = 3000000
private const val INSERTION_CUTOFF = 16
private const val SORT_STRIDE = 1009

private fun insertionSort(a: IntArray, lo: Int, hi: Int) {
    for (i in lo + 1..hi) {
        val v = a[i]
        var j = i - 1
        while (j >= lo && a[j] > v) {
            a[j + 1] = a[j]
            j--
        }
        a[j + 1] = v
    }
}

private fun quickSort(a: IntArray, loStart: Int, hiStart: Int) {
    var lo = loStart
    var hi = hiStart
    while (hi - lo >= INSERTION_CUTOFF) {
        val pivot = a[lo + (hi - lo) / 2]
        var i = lo
        var j = hi
        while (i <= j) {
            while (a[i] < pivot) i++
            while (a[j] > pivot) j--
            if (i <= j) {
                val t = a[i]
                a[i] = a[j]
                a[j] = t
                i++
                j--
            }
        }
        // recurse into the smaller part so stack depth stays logarithmic
        if (j - lo < hi - i) {
            quickSort(a, lo, j)
            lo = i
        } else {
            quickSort(a, i, hi)
            hi = j
        }
    }
    insertionSort(a, lo, hi)
}

fun testSort(): Long {
    val keys = IntArray(SORT_COUNT)
    val r = Rng(RNG_SEED)
    for (i in 0 until SORT_COUNT) keys[i] = r.key31()
    quickSort(keys, 0, SORT_COUNT - 1)
    var ordered = 1L
    for (i in 1 until SORT_COUNT) {
        if (keys[i - 1] > keys[i]) ordered = 0L
    }
    var h = MIX_SEED
    var i = 0
    while (i < SORT_COUNT) {
        h = mix(h, keys[i].toLong())
        i += SORT_STRIDE
    }
    return mix(h, ordered)
}
