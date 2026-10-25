package main

const (
	sortCount       = 3000000
	insertionCutoff = 16
	sortStride      = 1009
)

func insertionSort(a []uint32, lo, hi int) {
	for i := lo + 1; i <= hi; i++ {
		v := a[i]
		j := i - 1
		for j >= lo && a[j] > v {
			a[j+1] = a[j]
			j--
		}
		a[j+1] = v
	}
}

func quickSort(a []uint32, lo, hi int) {
	for hi-lo >= insertionCutoff {
		pivot := a[lo+(hi-lo)/2]
		i := lo
		j := hi
		for i <= j {
			for a[i] < pivot {
				i++
			}
			for a[j] > pivot {
				j--
			}
			if i <= j {
				t := a[i]
				a[i] = a[j]
				a[j] = t
				i++
				j--
			}
		}
		// recurse into the smaller part so stack depth stays logarithmic
		if j-lo < hi-i {
			quickSort(a, lo, j)
			lo = i
		} else {
			quickSort(a, i, hi)
			hi = j
		}
	}
	insertionSort(a, lo, hi)
}

func testSort() uint64 {
	keys := make([]uint32, sortCount)
	r := newRng(rngSeed)
	for i := 0; i < sortCount; i++ {
		keys[i] = r.key31()
	}
	quickSort(keys, 0, sortCount-1)
	var ordered uint64 = 1
	for i := 1; i < sortCount; i++ {
		if keys[i-1] > keys[i] {
			ordered = 0
		}
	}
	h := mixSeed
	for i := 0; i < sortCount; i += sortStride {
		h = mix(h, uint64(keys[i]))
	}
	return mix(h, ordered)
}
