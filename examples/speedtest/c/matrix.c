#include "tests.h"

enum { matN = 900, matCells = matN * matN, spectralN = 1500, spectralIters = 10 };

static const double spectralScale = 1.0e6;
static u32 matA[matCells];
static u32 matB[matCells];
static u32 matC[matCells];
static double specU[spectralN];
static double specV[spectralN];
static double specT[spectralN];

static void multiply(void) {
    for (u32 i = 0; i < matN; i++) {
        for (u32 k = 0; k < matN; k++) {
            u32 aik = matA[i * matN + k];
            for (u32 j = 0; j < matN; j++) matC[i * matN + j] += aik * matB[k * matN + j];
        }
    }
}

u64 testMatmul(void) {
    Rng r;
    rngInit(&r, rngSeed);
    for (u32 i = 0; i < matCells; i++) matA[i] = rngKey31(&r);
    for (u32 i = 0; i < matCells; i++) matB[i] = rngKey31(&r);
    multiply();
    u64 h = mixSeed;
    for (u32 i = 0; i < matCells; i++) h = mix(h, matC[i]);
    return h;
}

static double coef(u32 i, u32 j) {
    return 1.0 / (double)((i + j) * (i + j + 1) / 2 + i + 1);
}

static void mulAv(const double *v, double *out) {
    for (u32 i = 0; i < spectralN; i++) {
        double s = 0.0;
        for (u32 j = 0; j < spectralN; j++) s += coef(i, j) * v[j];
        out[i] = s;
    }
}

static void mulAtv(const double *v, double *out) {
    for (u32 i = 0; i < spectralN; i++) {
        double s = 0.0;
        for (u32 j = 0; j < spectralN; j++) s += coef(j, i) * v[j];
        out[i] = s;
    }
}

static void mulAtAv(const double *v, double *out) {
    mulAv(v, specT);
    mulAtv(specT, out);
}

u64 testSpectral(void) {
    for (u32 i = 0; i < spectralN; i++) specU[i] = 1.0;
    for (u32 it = 0; it < spectralIters; it++) {
        mulAtAv(specU, specV);
        mulAtAv(specV, specU);
    }
    double vbv = 0.0;
    double vv = 0.0;
    for (u32 i = 0; i < spectralN; i++) {
        vbv += specU[i] * specV[i];
        vv += specV[i] * specV[i];
    }
    return mix(mix(mixSeed, (u64)(vbv * spectralScale)), (u64)(vv * spectralScale));
}
