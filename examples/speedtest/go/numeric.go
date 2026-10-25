package main

const (
	sieveLimit   = 40000000
	collatzLimit = 1500000
	fibN         = 42
	queensN      = 14
	leibnizTerms = 150000000
	leibnizScale = 4.0e9
)

func testSieve() uint64 {
	composite := make([]byte, sieveLimit+1)
	for i := 2; i*i <= sieveLimit; i++ {
		if composite[i] != 0 {
			continue
		}
		for j := i * i; j <= sieveLimit; j += i {
			composite[j] = 1
		}
	}
	var count uint64
	for i := 2; i <= sieveLimit; i++ {
		if composite[i] == 0 {
			count++
		}
	}
	return mix(mixSeed, count)
}

func testCollatz() uint64 {
	var bestStart, bestSteps uint64
	for start := uint64(1); start <= collatzLimit; start++ {
		n := start
		var steps uint64
		for n != 1 {
			if n&1 == 1 {
				n = 3*n + 1
			} else {
				n >>= 1
			}
			steps++
		}
		if steps > bestSteps {
			bestSteps = steps
			bestStart = start
		}
	}
	return mix(mix(mixSeed, bestStart), bestSteps)
}

func fib(n uint64) uint64 {
	if n < 2 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func testFib() uint64 {
	return mix(mixSeed, fib(fibN))
}

func solveQueens(cols, diagL, diagR, full uint32) uint64 {
	if cols == full {
		return 1
	}
	var count uint64
	avail := full &^ (cols | diagL | diagR)
	for avail != 0 {
		bit := avail & (^avail + 1)
		avail ^= bit
		count += solveQueens(cols|bit, ((diagL|bit)<<1)&full, (diagR|bit)>>1, full)
	}
	return count
}

func testQueens() uint64 {
	full := uint32(1)<<queensN - 1
	return mix(mixSeed, solveQueens(0, 0, 0, full))
}

func testLeibniz() uint64 {
	sum := 0.0
	sign := 1.0
	for k := uint64(0); k < leibnizTerms; k++ {
		sum += sign / float64(2*k+1)
		sign = -sign
	}
	return mix(mixSeed, uint64(sum*leibnizScale))
}
