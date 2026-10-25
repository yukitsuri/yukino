package main

const (
	factN           = 20000
	limbCap         = 10000
	limbBase uint64 = 1000000000
)

func testBigfact() uint64 {
	limbs := make([]uint32, limbCap)
	used := 1
	limbs[0] = 1
	for m := uint64(2); m <= factN; m++ {
		var carry uint64
		for i := 0; i < used; i++ {
			t := uint64(limbs[i])*m + carry
			limbs[i] = uint32(t % limbBase)
			carry = t / limbBase
		}
		for carry != 0 {
			limbs[used] = uint32(carry % limbBase)
			used++
			carry /= limbBase
		}
	}
	h := mixSeed
	for i := 0; i < used; i++ {
		h = mix(h, uint64(limbs[i]))
	}
	return mix(h, uint64(used))
}
