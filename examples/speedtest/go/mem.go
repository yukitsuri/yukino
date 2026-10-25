package main

const (
	memSeqCount         = 12000000
	memRandCount        = 8000000
	memRandSteps        = 3000000
	memTreeNodes        = 1<<22 - 1
	noChild      uint32 = 0xffffffff
	treeValMul   uint32 = 2654435761
	memSeqMul    uint64 = 0x9e3779b97f4a7c15
)

type Tree struct {
	left  []uint32
	right []uint32
	val   []uint32
}

func (t *Tree) sum(node uint32) uint64 {
	if node == noChild {
		return 0
	}
	return uint64(t.val[node]) + t.sum(t.left[node]) + t.sum(t.right[node])
}

func testMemSeq() uint64 {
	buf := make([]uint64, memSeqCount)
	for i := 0; i < memSeqCount; i++ {
		buf[i] = uint64(i) * memSeqMul
	}
	var sum uint64
	for i := 0; i < memSeqCount; i++ {
		sum += buf[i] ^ uint64(i)
	}
	return mix(mixSeed, sum)
}

func testMemRand() uint64 {
	perm := make([]uint32, memRandCount)
	for i := 0; i < memRandCount; i++ {
		perm[i] = uint32(i)
	}
	r := newRng(rngSeed)
	// sattolo shuffle gives one cycle so every step is a new cache miss
	for i := memRandCount - 1; i > 0; i-- {
		j := int(r.below(uint64(i)))
		perm[i], perm[j] = perm[j], perm[i]
	}
	var p uint32
	var sum uint64
	for i := 0; i < memRandSteps; i++ {
		p = perm[p]
		sum += uint64(p)
	}
	return mix(mixSeed, sum)
}

func testMemTree() uint64 {
	t := &Tree{
		left:  make([]uint32, memTreeNodes),
		right: make([]uint32, memTreeNodes),
		val:   make([]uint32, memTreeNodes),
	}
	for i := 0; i < memTreeNodes; i++ {
		l := 2*i + 1
		r := 2*i + 2
		t.left[i] = noChild
		t.right[i] = noChild
		if l < memTreeNodes {
			t.left[i] = uint32(l)
		}
		if r < memTreeNodes {
			t.right[i] = uint32(r)
		}
		t.val[i] = uint32(i) * treeValMul
	}
	return mix(mixSeed, t.sum(0))
}
