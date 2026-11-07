package main

const (
	mandelSize    int64  = 1000
	mandelMaxIter uint64 = 500
	mandelFrac           = 24
)

func iterations(cr, ci int64) uint64 {
	const limit int64 = 4 << mandelFrac
	var zr, zi int64
	var n uint64
	for n < mandelMaxIter {
		zr2 := (zr * zr) >> mandelFrac
		zi2 := (zi * zi) >> mandelFrac
		if zr2+zi2 > limit {
			break
		}
		zi = ((2 * zr * zi) >> mandelFrac) + ci
		zr = zr2 - zi2 + cr
		n++
	}
	return n
}

func testMandel() uint64 {
	const one int64 = 1 << mandelFrac
	const x0 = -2 * one
	const y0 = -3 * one / 2
	const step = 3 * one / mandelSize
	var total uint64
	for py := int64(0); py < mandelSize; py++ {
		for px := int64(0); px < mandelSize; px++ {
			total += iterations(x0+px*step, y0+py*step)
		}
	}
	return mix(mixSeed, total)
}
