import kotlin.system.exitProcess

private const val EXIT_USAGE = 64
private const val EXIT_UNKNOWN = 65

private class Test(val name: String, val run: () -> Long)

private fun testNop(): Long = 0L

private val tests = arrayOf(
    Test("nop", ::testNop),
    Test("sieve", ::testSieve),
    Test("collatz", ::testCollatz),
    Test("fib", ::testFib),
    Test("queens", ::testQueens),
    Test("leibniz", ::testLeibniz),
    Test("matmul", ::testMatmul),
    Test("spectral", ::testSpectral),
    Test("mandel", ::testMandel),
    Test("sort", ::testSort),
    Test("hashmap", ::testHashmap),
    Test("sha256", ::testSha256),
    Test("crc32", ::testCrc32),
    Test("fnv", ::testFnv),
    Test("chacha", ::testChacha),
    Test("bigfact", ::testBigfact),
    Test("bf", ::testBf),
    Test("memseq", ::testMemSeq),
    Test("memrand", ::testMemRand),
    Test("memtree", ::testMemTree),
    Test("outbuf", ::testOutBuf),
    Test("outline", ::testOutLine),
)

fun main(args: Array<String>) {
    if (args.size != 1) exitProcess(EXIT_USAGE)
    for (t in tests) {
        if (t.name != args[0]) continue
        val start = System.nanoTime()
        val sum = t.run()
        report(sum, System.nanoTime() - start)
        return
    }
    exitProcess(EXIT_UNKNOWN)
}
