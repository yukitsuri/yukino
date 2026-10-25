#include "tests.hpp"

namespace {

constexpr u32 matN = 900;
constexpr u32 matCells = matN * matN;
constexpr u32 spectralN = 1500;
constexpr u32 spectralIters = 10;
constexpr double spectralScale = 1.0e6;

u32 matA[matCells];
u32 matB[matCells];
u32 matC[matCells];
double specU[spectralN];
double specV[spectralN];
double specT[spectralN];

void multiply() {
    for (u32 i = 0; i < matN; i++) {
        for (u32 k = 0; k < matN; k++) {
            u32 aik = matA[i * matN + k];
            for (u32 j = 0; j < matN; j++) matC[i * matN + j] += aik * matB[k * matN + j];
        }
    }
}

double coef(u32 i, u32 j) {
    return 1.0 / static_cast<double>((i + j) * (i + j + 1) / 2 + i + 1);
}

void mulAv(const double *v, double *out) {
    for (u32 i = 0; i < spectralN; i++) {
        double s = 0.0;
        for (u32 j = 0; j < spectralN; j++) s += coef(i, j) * v[j];
        out[i] = s;
    }
}

void mulAtv(const double *v, double *out) {
    for (u32 i = 0; i < spectralN; i++) {
        double s = 0.0;
        for (u32 j = 0; j < spectralN; j++) s += coef(j, i) * v[j];
        out[i] = s;
    }
}

void mulAtAv(const double *v, double *out) {
    mulAv(v, specT);
    mulAtv(specT, out);
}

}

u64 testMatmul() {
    Rng r(rngSeed);
    for (u32 i = 0; i < matCells; i++) matA[i] = r.key31();
    for (u32 i = 0; i < matCells; i++) matB[i] = r.key31();
    multiply();
    u64 h = mixSeed;
    for (u32 i = 0; i < matCells; i++) h = mix(h, matC[i]);
    return h;
}

u64 testSpectral() {
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
    return mix(mix(mixSeed, static_cast<u64>(vbv * spectralScale)), static_cast<u64>(vv * spectralScale));
}
