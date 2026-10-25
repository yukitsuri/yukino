package main

import (
	"os"
	"time"
)

const (
	exitUsage   = 64
	exitUnknown = 65
)

type Test struct {
	name string
	run  func() uint64
}

func testNop() uint64 {
	return 0
}

var tests = []Test{
	{"nop", testNop},
	{"sieve", testSieve},
	{"collatz", testCollatz},
	{"fib", testFib},
	{"queens", testQueens},
	{"leibniz", testLeibniz},
	{"matmul", testMatmul},
	{"spectral", testSpectral},
	{"mandel", testMandel},
	{"sort", testSort},
	{"hashmap", testHashmap},
	{"sha256", testSha256},
	{"crc32", testCrc32},
	{"fnv", testFnv},
	{"chacha", testChacha},
	{"bigfact", testBigfact},
	{"bf", testBf},
	{"memseq", testMemSeq},
	{"memrand", testMemRand},
	{"memtree", testMemTree},
	{"outbuf", testOutBuf},
	{"outline", testOutLine},
}

func main() {
	if len(os.Args) != 2 {
		os.Exit(exitUsage)
	}
	for _, t := range tests {
		if t.name != os.Args[1] {
			continue
		}
		start := time.Now()
		sum := t.run()
		report(sum, uint64(time.Since(start).Nanoseconds()))
		return
	}
	os.Exit(exitUnknown)
}
