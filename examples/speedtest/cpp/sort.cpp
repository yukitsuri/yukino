#include "tests.hpp"

namespace {

constexpr i64 sortCount = 3000000;
constexpr i64 insertionCutoff = 16;
constexpr i64 sortStride = 1009;

u32 keys[sortCount];

void insertionSort(u32 *a, i64 lo, i64 hi) {
    for (i64 i = lo + 1; i <= hi; i++) {
        u32 v = a[i];
        i64 j = i - 1;
        while (j >= lo && a[j] > v) {
            a[j + 1] = a[j];
            j--;
        }
        a[j + 1] = v;
    }
}

void quickSort(u32 *a, i64 lo, i64 hi) {
    while (hi - lo >= insertionCutoff) {
        u32 pivot = a[lo + (hi - lo) / 2];
        i64 i = lo;
        i64 j = hi;
        while (i <= j) {
            while (a[i] < pivot) i++;
            while (a[j] > pivot) j--;
            if (i <= j) {
                u32 t = a[i];
                a[i] = a[j];
                a[j] = t;
                i++;
                j--;
            }
        }
        // recurse into the smaller part so stack depth stays logarithmic
        if (j - lo < hi - i) {
            quickSort(a, lo, j);
            lo = i;
        } else {
            quickSort(a, i, hi);
            hi = j;
        }
    }
    insertionSort(a, lo, hi);
}

}

u64 testSort() {
    Rng r(rngSeed);
    for (i64 i = 0; i < sortCount; i++) keys[i] = r.key31();
    quickSort(keys, 0, sortCount - 1);
    u64 ordered = 1;
    for (i64 i = 1; i < sortCount; i++) {
        if (keys[i - 1] > keys[i]) ordered = 0;
    }
    u64 h = mixSeed;
    for (i64 i = 0; i < sortCount; i += sortStride) h = mix(h, keys[i]);
    return mix(h, ordered);
}
