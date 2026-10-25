package main

import "os"

const (
	mixSeed        uint64 = 0xcbf29ce484222325
	mixPrime       uint64 = 0x100000001b3
	rngSeed        uint64 = 0x9e3779b97f4a7c15
	rngMul         uint64 = 0x2545f4914f6cdd1d
	rngShiftA             = 12
	rngShiftB             = 25
	rngShiftC             = 27
	key31Shift            = 33
	fillMul        uint32 = 167
	fillShift             = 9
	decimalBase    uint64 = 10
	maxDigits             = 20
	reportBytes           = 64
	exitWriteFails        = 2
)

type Rng struct {
	state uint64
}

func newRng(seed uint64) Rng {
	return Rng{state: seed}
}

func (r *Rng) next() uint64 {
	x := r.state
	x ^= x >> rngShiftA
	x ^= x << rngShiftB
	x ^= x >> rngShiftC
	r.state = x
	return x * rngMul
}

func (r *Rng) key31() uint32 {
	return uint32(r.next() >> key31Shift)
}

func (r *Rng) below(n uint64) uint64 {
	return (r.next() >> 1) % n
}

func mix(h uint64, v uint64) uint64 {
	return (h ^ v) * mixPrime
}

func fillByte(i uint32) byte {
	return byte(i*fillMul + i>>fillShift)
}

func writeOut(buf []byte) {
	if _, err := os.Stdout.Write(buf); err != nil {
		os.Exit(exitWriteFails)
	}
}

func fmtU64(v uint64, out []byte) int {
	var tmp [maxDigits]byte
	n := 0
	for {
		tmp[n] = byte('0' + v%decimalBase)
		n++
		v /= decimalBase
		if v == 0 {
			break
		}
	}
	for i := 0; i < n; i++ {
		out[i] = tmp[n-1-i]
	}
	return n
}

func report(checksum uint64, ns uint64) {
	var line [reportBytes]byte
	line[0] = 'R'
	line[1] = ' '
	n := 2
	n += fmtU64(checksum, line[n:])
	line[n] = ' '
	n++
	n += fmtU64(ns, line[n:])
	line[n] = '\n'
	n++
	if _, err := os.Stderr.Write(line[:n]); err != nil {
		os.Exit(exitWriteFails)
	}
}
