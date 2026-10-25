package main

const (
	hashBufBytes         = 1 << 22
	shaPasses            = 16
	crcPasses            = 32
	fnvPasses            = 64
	shaBlockBytes        = 64
	shaRounds            = 64
	crcPoly       uint32 = 0xedb88320
)

var shaInit = [8]uint32{0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19}

var shaK = [shaRounds]uint32{
	0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
	0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
	0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
	0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
	0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
	0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
	0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
	0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
}

func makeBuf() []byte {
	buf := make([]byte, hashBufBytes)
	for i := 0; i < hashBufBytes; i++ {
		buf[i] = fillByte(uint32(i))
	}
	return buf
}

func ror(x uint32, n uint32) uint32 {
	return x>>n | x<<(32-n)
}

func shaBlock(st *[8]uint32, p []byte) {
	var w [shaRounds]uint32
	for i := 0; i < 16; i++ {
		w[i] = uint32(p[4*i])<<24 | uint32(p[4*i+1])<<16 | uint32(p[4*i+2])<<8 | uint32(p[4*i+3])
	}
	for i := 16; i < shaRounds; i++ {
		s0 := ror(w[i-15], 7) ^ ror(w[i-15], 18) ^ (w[i-15] >> 3)
		s1 := ror(w[i-2], 17) ^ ror(w[i-2], 19) ^ (w[i-2] >> 10)
		w[i] = w[i-16] + s0 + w[i-7] + s1
	}
	a, b, c, d, e, f, g, h := st[0], st[1], st[2], st[3], st[4], st[5], st[6], st[7]
	for i := 0; i < shaRounds; i++ {
		t1 := h + (ror(e, 6) ^ ror(e, 11) ^ ror(e, 25)) + ((e & f) ^ (^e & g)) + shaK[i] + w[i]
		t2 := (ror(a, 2) ^ ror(a, 13) ^ ror(a, 22)) + ((a & b) ^ (a & c) ^ (b & c))
		h = g
		g = f
		f = e
		e = d + t1
		d = c
		c = b
		b = a
		a = t1 + t2
	}
	st[0] += a
	st[1] += b
	st[2] += c
	st[3] += d
	st[4] += e
	st[5] += f
	st[6] += g
	st[7] += h
}

func testSha256() uint64 {
	buf := makeBuf()
	st := shaInit
	for pass := 0; pass < shaPasses; pass++ {
		for off := 0; off < hashBufBytes; off += shaBlockBytes {
			shaBlock(&st, buf[off:off+shaBlockBytes])
		}
	}
	var last [shaBlockBytes]byte
	bits := uint64(shaPasses) * hashBufBytes * 8
	last[0] = 0x80
	for k := 0; k < 8; k++ {
		last[shaBlockBytes-1-k] = byte(bits >> (8 * k))
	}
	shaBlock(&st, last[:])
	h := mixSeed
	for i := 0; i < 8; i++ {
		h = mix(h, uint64(st[i]))
	}
	return h
}

func testCrc32() uint64 {
	buf := makeBuf()
	var table [256]uint32
	for n := 0; n < 256; n++ {
		c := uint32(n)
		for k := 0; k < 8; k++ {
			if c&1 == 1 {
				c = (c >> 1) ^ crcPoly
			} else {
				c >>= 1
			}
		}
		table[n] = c
	}
	crc := uint32(0xffffffff)
	for pass := 0; pass < crcPasses; pass++ {
		for i := 0; i < hashBufBytes; i++ {
			crc = table[(crc^uint32(buf[i]))&0xff] ^ (crc >> 8)
		}
	}
	return mix(mixSeed, uint64(^crc))
}

func testFnv() uint64 {
	buf := makeBuf()
	h := mixSeed
	for pass := 0; pass < fnvPasses; pass++ {
		for i := 0; i < hashBufBytes; i++ {
			h = mix(h, uint64(buf[i]))
		}
	}
	return h
}
