package main

const (
	mapBits           = 22
	mapSize           = 1 << mapBits
	mapInserts        = 2000000
	mapMisses         = 1000000
	mapHashMul uint64 = 0x9e3779b97f4a7c15
	mapMissing uint64 = ^uint64(0)
	missSeed   uint64 = 0x2545f4914f6cdd1d
)

type Table struct {
	keys []uint64
	vals []uint64
}

func newTable() *Table {
	return &Table{keys: make([]uint64, mapSize), vals: make([]uint64, mapSize)}
}

func slotOf(key uint64) uint32 {
	return uint32((key * mapHashMul) >> (64 - mapBits))
}

func (t *Table) put(key, val uint64) {
	s := slotOf(key)
	for t.keys[s] != 0 && t.keys[s] != key {
		s = (s + 1) & (mapSize - 1)
	}
	t.keys[s] = key
	t.vals[s] = val
}

func (t *Table) get(key uint64) uint64 {
	s := slotOf(key)
	for t.keys[s] != 0 {
		if t.keys[s] == key {
			return t.vals[s]
		}
		s = (s + 1) & (mapSize - 1)
	}
	return mapMissing
}

func testHashmap() uint64 {
	table := newTable()
	r := newRng(rngSeed)
	for i := uint64(0); i < mapInserts; i++ {
		table.put(r.next()|1, i)
	}
	again := newRng(rngSeed)
	var sum uint64
	for i := 0; i < mapInserts; i++ {
		sum += table.get(again.next() | 1)
	}
	miss := newRng(missSeed)
	var misses uint64
	for i := 0; i < mapMisses; i++ {
		v := table.get(miss.next() | 1)
		if v == mapMissing {
			misses++
		} else {
			sum += v
		}
	}
	return mix(mix(mixSeed, sum), misses)
}
