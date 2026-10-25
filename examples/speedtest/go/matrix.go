package main

const (
	matN          = 900
	matCells      = matN * matN
	spectralN     = 1500
	spectralIters = 10
	spectralScale = 1.0e6
)

func multiply(a, b, c []uint32) {
	for i := 0; i < matN; i++ {
		for k := 0; k < matN; k++ {
			aik := a[i*matN+k]
			brow := b[k*matN : (k+1)*matN]
			crow := c[i*matN : (i+1)*matN]
			for j := 0; j < matN; j++ {
				crow[j] += aik * brow[j]
			}
		}
	}
}

func testMatmul() uint64 {
	a := make([]uint32, matCells)
	b := make([]uint32, matCells)
	c := make([]uint32, matCells)
	r := newRng(rngSeed)
	for i := 0; i < matCells; i++ {
		a[i] = r.key31()
	}
	for i := 0; i < matCells; i++ {
		b[i] = r.key31()
	}
	multiply(a, b, c)
	h := mixSeed
	for i := 0; i < matCells; i++ {
		h = mix(h, uint64(c[i]))
	}
	return h
}

func coef(i, j uint32) float64 {
	return 1.0 / float64((i+j)*(i+j+1)/2+i+1)
}

func mulAv(v, out []float64) {
	for i := 0; i < spectralN; i++ {
		s := 0.0
		for j := 0; j < spectralN; j++ {
			s += float64(coef(uint32(i), uint32(j)) * v[j])
		}
		out[i] = s
	}
}

func mulAtv(v, out []float64) {
	for i := 0; i < spectralN; i++ {
		s := 0.0
		for j := 0; j < spectralN; j++ {
			s += float64(coef(uint32(j), uint32(i)) * v[j])
		}
		out[i] = s
	}
}

func mulAtAv(v, out, tmp []float64) {
	mulAv(v, tmp)
	mulAtv(tmp, out)
}

func testSpectral() uint64 {
	u := make([]float64, spectralN)
	v := make([]float64, spectralN)
	t := make([]float64, spectralN)
	for i := 0; i < spectralN; i++ {
		u[i] = 1.0
	}
	for it := 0; it < spectralIters; it++ {
		mulAtAv(u, v, t)
		mulAtAv(v, u, t)
	}
	vbv := 0.0
	vv := 0.0
	for i := 0; i < spectralN; i++ {
		vbv += float64(u[i] * v[i])
		vv += float64(v[i] * v[i])
	}
	return mix(mix(mixSeed, uint64(vbv*spectralScale)), uint64(vv*spectralScale))
}
