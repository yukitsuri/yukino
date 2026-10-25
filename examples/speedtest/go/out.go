package main

const (
	outBufSize     = 65536
	outLines       = 3000000
	outSingleLines = 300000
	maxLineBytes   = 64
)

func fmtLine(dst []byte, index, value uint64) int {
	n := fmtU64(index, dst)
	dst[n] = ' '
	n++
	n += fmtU64(value, dst[n:])
	dst[n] = '\n'
	return n + 1
}

func testOutBuf() uint64 {
	r := newRng(rngSeed)
	buf := make([]byte, outBufSize)
	used := 0
	var total uint64
	for i := uint64(0); i < outLines; i++ {
		if used+maxLineBytes > outBufSize {
			writeOut(buf[:used])
			total += uint64(used)
			used = 0
		}
		used += fmtLine(buf[used:], i, r.next())
	}
	writeOut(buf[:used])
	return mix(mixSeed, total+uint64(used))
}

func testOutLine() uint64 {
	r := newRng(rngSeed)
	var line [maxLineBytes]byte
	var total uint64
	for i := uint64(0); i < outSingleLines; i++ {
		n := fmtLine(line[:], i, r.next())
		writeOut(line[:n])
		total += uint64(n)
	}
	return mix(mixSeed, total)
}
