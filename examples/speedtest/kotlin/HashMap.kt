private const val MAP_BITS = 22
private const val MAP_SIZE = 1 shl MAP_BITS
private const val MAP_INSERTS = 2000000L
private const val MAP_MISSES = 1000000L
private const val MAP_HASH_MUL: Long = -0x61c8864680b583ebL
private const val MAP_MISSING = -1L
private const val MISS_SEED: Long = 0x2545f4914f6cdd1dL

private class Table {
    val keys = LongArray(MAP_SIZE)
    val vals = LongArray(MAP_SIZE)

    fun slotOf(key: Long): Int = ((key * MAP_HASH_MUL) ushr (64 - MAP_BITS)).toInt()

    fun put(key: Long, value: Long) {
        var s = slotOf(key)
        while (keys[s] != 0L && keys[s] != key) s = (s + 1) and (MAP_SIZE - 1)
        keys[s] = key
        vals[s] = value
    }

    fun get(key: Long): Long {
        var s = slotOf(key)
        while (keys[s] != 0L) {
            if (keys[s] == key) return vals[s]
            s = (s + 1) and (MAP_SIZE - 1)
        }
        return MAP_MISSING
    }
}

fun testHashmap(): Long {
    val table = Table()
    val r = Rng(RNG_SEED)
    for (i in 0 until MAP_INSERTS) table.put(r.next() or 1L, i)
    val again = Rng(RNG_SEED)
    var sum = 0L
    for (i in 0 until MAP_INSERTS) sum += table.get(again.next() or 1L)
    val miss = Rng(MISS_SEED)
    var misses = 0L
    for (i in 0 until MAP_MISSES) {
        val v = table.get(miss.next() or 1L)
        if (v == MAP_MISSING) misses++ else sum += v
    }
    return mix(mix(MIX_SEED, sum), misses)
}
