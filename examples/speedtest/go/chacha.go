package main

const (
	chachaBlocks       = 2000000
	chachaWords        = 16
	chachaDoubleRounds = 10
)

var chachaConst = [4]uint32{0x61707865, 0x3320646e, 0x79622d32, 0x6b206574}

func rol(x uint32, n uint32) uint32 {
	return x<<n | x>>(32-n)
}

func quarter(s *[chachaWords]uint32, a, b, c, d int) {
	s[a] += s[b]
	s[d] = rol(s[d]^s[a], 16)
	s[c] += s[d]
	s[b] = rol(s[b]^s[c], 12)
	s[a] += s[b]
	s[d] = rol(s[d]^s[a], 8)
	s[c] += s[d]
	s[b] = rol(s[b]^s[c], 7)
}

func chachaBlock(in *[chachaWords]uint32, out *[chachaWords]uint32) {
	*out = *in
	for r := 0; r < chachaDoubleRounds; r++ {
		quarter(out, 0, 4, 8, 12)
		quarter(out, 1, 5, 9, 13)
		quarter(out, 2, 6, 10, 14)
		quarter(out, 3, 7, 11, 15)
		quarter(out, 0, 5, 10, 15)
		quarter(out, 1, 6, 11, 12)
		quarter(out, 2, 7, 8, 13)
		quarter(out, 3, 4, 9, 14)
	}
	for i := 0; i < chachaWords; i++ {
		out[i] += in[i]
	}
}

func testChacha() uint64 {
	var st, out [chachaWords]uint32
	r := newRng(rngSeed)
	for i := 0; i < 4; i++ {
		st[i] = chachaConst[i]
	}
	for i := 4; i < chachaWords; i++ {
		st[i] = uint32(r.next())
	}
	var acc uint32
	for c := uint32(0); c < chachaBlocks; c++ {
		st[12] = c
		chachaBlock(&st, &out)
		for i := 0; i < chachaWords; i++ {
			acc ^= out[i]
		}
	}
	return mix(mixSeed, uint64(acc))
}
