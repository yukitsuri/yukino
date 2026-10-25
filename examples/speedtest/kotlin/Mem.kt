private const val MEM_SEQ_COUNT = 12000000
private const val MEM_RAND_COUNT = 8000000
private const val MEM_RAND_STEPS = 3000000
private const val MEM_TREE_NODES = (1 shl 22) - 1
private const val NO_CHILD = -1
private const val TREE_VAL_MUL: Int = 2654435761L.toInt()
private const val MEM_SEQ_MUL: Long = -0x61c8864680b583ebL
private const val U32_MASK = 0xFFFFFFFFL

private class Tree {
    val left = IntArray(MEM_TREE_NODES)
    val right = IntArray(MEM_TREE_NODES)
    val value = IntArray(MEM_TREE_NODES)

    fun sum(node: Int): Long {
        if (node == NO_CHILD) return 0L
        return (value[node].toLong() and U32_MASK) + sum(left[node]) + sum(right[node])
    }
}

fun testMemSeq(): Long {
    val buf = LongArray(MEM_SEQ_COUNT)
    for (i in 0 until MEM_SEQ_COUNT) buf[i] = i.toLong() * MEM_SEQ_MUL
    var sum = 0L
    for (i in 0 until MEM_SEQ_COUNT) sum += buf[i] xor i.toLong()
    return mix(MIX_SEED, sum)
}

fun testMemRand(): Long {
    val perm = IntArray(MEM_RAND_COUNT)
    for (i in 0 until MEM_RAND_COUNT) perm[i] = i
    val r = Rng(RNG_SEED)
    // sattolo shuffle gives one cycle so every step is a new cache miss
    for (i in MEM_RAND_COUNT - 1 downTo 1) {
        val j = r.below(i.toLong()).toInt()
        val t = perm[i]
        perm[i] = perm[j]
        perm[j] = t
    }
    var p = 0
    var sum = 0L
    for (i in 0 until MEM_RAND_STEPS) {
        p = perm[p]
        sum += p.toLong()
    }
    return mix(MIX_SEED, sum)
}

fun testMemTree(): Long {
    val tree = Tree()
    for (i in 0 until MEM_TREE_NODES) {
        val l = 2 * i + 1
        val r = 2 * i + 2
        tree.left[i] = if (l < MEM_TREE_NODES) l else NO_CHILD
        tree.right[i] = if (r < MEM_TREE_NODES) r else NO_CHILD
        tree.value[i] = i * TREE_VAL_MUL
    }
    return mix(MIX_SEED, tree.sum(0))
}
