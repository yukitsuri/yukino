#define _GNU_SOURCE
#include <fcntl.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <sys/stat.h>

enum {
    maxLangs = 8,
    maxTests = 32,
    maxArgs = 16,
    readCap = 4096,
    timeoutSec = 180,
    defaultRuns = 3,
    labelWidth = 9,
    cellWidth = 7,
    cmdCap = 512
};

typedef unsigned long long u64;
typedef enum { kindNop, kindCpu, kindIo, kindMem } Kind;
typedef enum { metricInternal, metricWall, metricRss } Metric;
typedef enum { unitSec, unitMem, unitRatio, unitSize } Unit;
typedef enum { statusNone, statusOk, statusFail, statusMismatch } Status;
static const unsigned allKinds = (1u << kindCpu) | (1u << kindIo) | (1u << kindMem);

typedef enum { langOff, langMissing, langReady, langBuildFailed } LangState;

typedef struct {
    const char *name;
    Kind kind;
} TestDef;

typedef struct {
    const char *name;
    const char *color;
    const char *probe;
    const char *warm;
    const char *build;
    const char *runArgv[maxArgs];
} Lang;

typedef struct {
    Status status;
    double wall;
    double internal;
    long rssKb;
    u64 checksum;
} Cell;

typedef struct {
    double v[maxLangs];
    int has[maxLangs];
    int bad[maxLangs];
} Row;

static const char *const styleReset = "\033[0m";
static const char *const styleBold = "\033[1m";
static const char *const styleDim = "\033[2m";
static const char *const styleRed = "\033[91m";
static const char *const styleGreen = "\033[92m";

static const TestDef testDefs[] = {
    {"nop", kindNop},        {"sieve", kindCpu},   {"collatz", kindCpu}, {"fib", kindCpu},      {"queens", kindCpu},
    {"leibniz", kindCpu},    {"matmul", kindCpu},  {"spectral", kindCpu}, {"mandel", kindCpu},  {"sort", kindCpu},
    {"hashmap", kindCpu},    {"sha256", kindCpu},  {"crc32", kindCpu},   {"fnv", kindCpu},      {"chacha", kindCpu},
    {"bigfact", kindCpu},    {"bf", kindCpu},      {"outbuf", kindIo},   {"outline", kindIo},   {"memseq", kindMem},
    {"memrand", kindMem},    {"memtree", kindMem},
};

static const Lang langs[] = {
    {"c", "\033[94m", "command -v ${CC:-gcc}", NULL,
     "${CC:-gcc} -std=c11 -O3 -march=native -flto -ffp-contract=off -o build/c/bin c/*.c",
     {"build/c/bin", NULL}},
    {"cpp", "\033[95m", "command -v ${CXX:-g++}", NULL,
     "${CXX:-g++} -std=c++17 -O3 -march=native -flto -ffp-contract=off -fno-exceptions -fno-rtti -o build/cpp/bin cpp/*.cpp",
     {"build/cpp/bin", NULL}},
    {"zig", "\033[93m", "command -v zig",
     "zig build-exe zig/main.zig -O ReleaseFast -lc -mcpu=native -fsingle-threaded --cache-dir build/zig/warmcache -femit-bin=build/zig/warm",
     "zig build-exe zig/main.zig -O ReleaseFast -lc -mcpu=native -fsingle-threaded --cache-dir build/zig/cache -femit-bin=build/zig/bin",
     {"build/zig/bin", NULL}},
    {"rust", "\033[91m", "command -v rustc", NULL,
     "rustc --edition 2021 -C opt-level=3 -C lto=fat -C codegen-units=1 -C panic=abort -C target-cpu=native -o build/rust/bin rust/main.rs",
     {"build/rust/bin", NULL}},
    {"go", "\033[96m", "command -v go",
     "cd go && go build -gcflags=-B -trimpath -o /dev/null .",
     "cd go && printf 'package main\\n\\nconst buildId = \"%s\"\\n' \"$$\" > zzbuildid.go && "
     "go build -gcflags=-B -ldflags=\"-s -w\" -trimpath -o ../build/go/bin . ; rc=$?; rm -f zzbuildid.go; exit $rc",
     {"build/go/bin", NULL}},
    {"kotlin", "\033[92m", "command -v kotlinc && command -v java", NULL,
     "kotlinc kotlin/*.kt -include-runtime -d build/kotlin/out.jar -Xno-param-assertions -Xno-call-assertions -Xno-receiver-assertions",
     {"java", "-XX:+UseSerialGC", "-Xmx1g", "-jar", "build/kotlin/out.jar", NULL}},
    {"yukino", "\033[38;5;117m", "command -v ${YUKINO:-yukino-dbg} >/dev/null 2>&1 || command -v yukino",
     "YKN=\"${YUKINO:-yukino-dbg}\"; command -v \"$YKN\" >/dev/null 2>&1 || YKN=yukino; cd yukino && \"$YKN\" build release --no-out && rm -rf yuki-work/local/build yuki-work/local/cache",
     "YKN=\"${YUKINO:-yukino-dbg}\"; command -v \"$YKN\" >/dev/null 2>&1 || YKN=yukino; cd yukino && \"$YKN\" build release --no-out && mkdir -p ../build/yukino && cp yuki-work/local/build/release/*/*/* ../build/yukino/bin",
     {"build/yukino/bin", NULL}},
};

static const int langCount = sizeof langs / sizeof langs[0];
static const int testCount = sizeof testDefs / sizeof testDefs[0];

static Cell cells[maxTests][maxLangs];
static double buildSec[maxLangs];
static LangState langState[maxLangs];
static int testSelected[maxTests];
static int useColor = 0;
static int runs = defaultRuns;
static int verbose = 0;

static double nowSec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}

static void paint(const char *code) {
    if (useColor) fputs(code, stdout);
}

static void progress(const char *what, const char *name) {
    if (!useColor) return;
    fprintf(stderr, "\r\033[K%s %s", what, name);
    fflush(stderr);
}

static void clearProgress(void) {
    if (useColor) fputs("\r\033[K", stderr);
}

static void enterRoot(const char *argv0) {
    char path[PATH_MAX];
    ssize_t n = readlink("/proc/self/exe", path, sizeof path - 1);
    if (n > 0) {
        path[n] = '\0';
    } else {
        snprintf(path, sizeof path, "%s", argv0);
    }
    char *slash = strrchr(path, '/');
    if (slash == NULL) return;
    *slash = '\0';
    if (chdir(path) != 0) {
        fprintf(stderr, "cannot enter %s\n", path);
        exit(1);
    }
}

static int shell(const char *cmd, const char *log) {
    char full[cmdCap + PATH_MAX + 16];
    snprintf(full, sizeof full, "(%s) >%s 2>&1", cmd, log);
    return system(full);
}

static void buildLang(int l) {
    char cmd[cmdCap];
    char log[PATH_MAX];
    snprintf(cmd, sizeof cmd, "(%s) >/dev/null 2>&1", langs[l].probe);
    if (system(cmd) != 0) {
        langState[l] = langMissing;
        return;
    }
    snprintf(cmd, sizeof cmd, "rm -rf build/%s && mkdir -p build/%s", langs[l].name, langs[l].name);
    if (system(cmd) != 0) return;
    if (strcmp(langs[l].name, "yukino") == 0) system("rm -rf yukino/yuki-work/local/build yukino/yuki-work/local/cache");
    snprintf(log, sizeof log, "build/%s/build.log", langs[l].name);
    progress("build", langs[l].name);
    if (langs[l].warm != NULL) shell(langs[l].warm, "/dev/null");
    double start = nowSec();
    int rc = shell(langs[l].build, log);
    if (rc == 0 && strcmp(langs[l].name, "yukino") == 0) {
        FILE *f = fopen(log, "r");
        if (f != NULL) {
            char lineBuf[64] = {0};
            if (fgets(lineBuf, sizeof lineBuf, f) != NULL) {
                if (lineBuf[0] != '0') {
                    rc = 1;
                }
            }
            fclose(f);
        }
    }
    buildSec[l] = nowSec() - start;
    langState[l] = rc == 0 ? langReady : langBuildFailed;
}

typedef struct {
    double wall;
    double internal;
    long rssKb;
    u64 checksum;
} Sample;

static int parseReport(const char *buf, Sample *s) {
    int found = 0;
    const char *p = buf;
    while (*p != '\0') {
        u64 sum;
        u64 ns;
        if (sscanf(p, "R %llu %llu", &sum, &ns) == 2) {
            s->checksum = sum;
            s->internal = (double)ns / 1e9;
            found = 1;
        }
        const char *nl = strchr(p, '\n');
        if (nl == NULL) break;
        p = nl + 1;
    }
    return found;
}

static int runOnce(const char *const *argv, Sample *s) {
    int errPipe[2];
    if (pipe(errPipe) != 0) return 0;
    double start = nowSec();
    pid_t pid = fork();
    if (pid < 0) return 0;
    if (pid == 0) {
        // stdout goes to /dev/null so only the language side cost is measured
        int nul = open("/dev/null", O_RDWR);
        dup2(nul, STDIN_FILENO);
        dup2(nul, STDOUT_FILENO);
        dup2(errPipe[1], STDERR_FILENO);
        close(errPipe[0]);
        close(errPipe[1]);
        close(nul);
        alarm(timeoutSec);
        // posix execvp takes a non const argv
        execvp(argv[0], (char *const *)argv);
        _exit(127);
    }
    close(errPipe[1]);
    char buf[readCap];
    char chunk[readCap];
    size_t len = 0;
    ssize_t n;
    while ((n = read(errPipe[0], chunk, sizeof chunk)) > 0) {
        size_t room = sizeof buf - 1 - len;
        size_t take = (size_t)n < room ? (size_t)n : room;
        memcpy(buf + len, chunk, take);
        len += take;
    }
    close(errPipe[0]);
    buf[len] = '\0';
    int status;
    struct rusage ru;
    if (wait4(pid, &status, 0, &ru) < 0) return 0;
    s->wall = nowSec() - start;
    s->rssKb = ru.ru_maxrss;
#ifdef __APPLE__
    s->rssKb /= 1024;
#endif
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) return 0;
    return parseReport(buf, s);
}

static void benchCell(int t, int l) {
    const char *argv[maxArgs];
    int n = 0;
    for (int i = 0; langs[l].runArgv[i] != NULL; i++) argv[n++] = langs[l].runArgv[i];
    argv[n++] = testDefs[t].name;
    argv[n] = NULL;
    Cell *c = &cells[t][l];
    c->status = statusOk;
    c->wall = 1e30;
    c->internal = 1e30;
    c->rssKb = 0;
    for (int i = 0; i < runs; i++) {
        Sample s;
        if (!runOnce(argv, &s) || (i > 0 && s.checksum != c->checksum)) {
            c->status = statusFail;
            return;
        }
        c->checksum = s.checksum;
        if (s.wall < c->wall) c->wall = s.wall;
        if (s.internal < c->internal) c->internal = s.internal;
        if (s.rssKb > c->rssKb) c->rssKb = s.rssKb;
    }
}

static void markMismatches(int t) {
    int ref = -1;
    for (int l = 0; l < langCount; l++) {
        if (cells[t][l].status != statusOk) continue;
        if (ref < 0) ref = l;
        if (cells[t][l].checksum != cells[t][ref].checksum) cells[t][l].status = statusMismatch;
    }
}

static double metricOf(const Cell *c, Metric m) {
    if (m == metricWall) return c->wall;
    if (m == metricRss) return (double)c->rssKb / 1024.0;
    return c->internal;
}

static void printCell(const Row *r, int l, Unit unit, double best) {
    char text[32];
    if (!r->has[l]) {
        snprintf(text, sizeof text, "-");
    } else if (unit == unitSec) {
        snprintf(text, sizeof text, "%.2fs", r->v[l]);
    } else if (unit == unitMem) {
        snprintf(text, sizeof text, "%.1fM", r->v[l]);
    } else if (unit == unitSize) {
        if (r->v[l] >= 1024.0) snprintf(text, sizeof text, "%.1fM", r->v[l] / 1024.0);
        else snprintf(text, sizeof text, "%.0fK", r->v[l]);
    } else {
        snprintf(text, sizeof text, "%.2fx", r->v[l]);
    }
    paint(r->bad[l] ? styleRed : langs[l].color);
    if (r->has[l] && r->v[l] == best) paint(styleBold);
    printf("%*s ", cellWidth, text);
    paint(styleReset);
}

static void printRow(const char *label, const Row *r, Unit unit) {
    double best = 1e30;
    for (int l = 0; l < langCount; l++) {
        if (r->has[l] && r->v[l] < best) best = r->v[l];
    }
    printf("%-*s ", labelWidth, label);
    int printed = 0;
    for (int l = 0; l < langCount; l++) {
        if (langState[l] == langReady) {
            if (printed > 0 && strcmp(langs[l].name, "yukino") == 0) {
                printf("%*s ", cellWidth, "");
            }
            printCell(r, l, unit, best);
            printed++;
        }
    }
    printf("\n");
}

static void printTitle(const char *title) {
    paint(styleDim);
    printf("%s", title);
    paint(styleReset);
    printf("\n");
}

static void fillRow(Row *r, int t, Metric m) {
    memset(r, 0, sizeof *r);
    for (int l = 0; l < langCount; l++) {
        Status st = cells[t][l].status;
        r->has[l] = st == statusOk || st == statusMismatch;
        r->bad[l] = st == statusMismatch;
        if (r->has[l]) r->v[l] = metricOf(&cells[t][l], m);
    }
}

static void printSection(const char *title, unsigned kindMask, Metric m, Unit unit) {
    int titled = 0;
    for (int t = 0; t < testCount; t++) {
        if (!testSelected[t] || !(kindMask & (1u << testDefs[t].kind))) continue;
        if (!titled) printTitle(title);
        titled = 1;
        Row r;
        fillRow(&r, t, m);
        printRow(testDefs[t].name, &r, unit);
    }
}

typedef struct {
    Row wall;
    Row cpu;
    Row io;
    Row peak;
    Row score;
} Totals;

static double bestInternal(int t) {
    double best = 1e30;
    for (int j = 0; j < langCount; j++) {
        if (cells[t][j].status == statusOk && cells[t][j].internal < best) best = cells[t][j].internal;
    }
    return best;
}

static int langComplete(int l) {
    if (langState[l] != langReady) return 0;
    for (int t = 0; t < testCount; t++) {
        if (!testSelected[t]) continue;
        if (cells[t][l].status == statusFail || cells[t][l].status == statusNone) return 0;
    }
    return 1;
}

static void addTotals(int l, Totals *tot) {
    double logSum = 0.0;
    int scored = 0;
    for (int t = 0; t < testCount; t++) {
        if (!testSelected[t]) continue;
        const Cell *c = &cells[t][l];
        double ramMb = (double)c->rssKb / 1024.0;
        tot->wall.v[l] += c->wall;
        if (ramMb > tot->peak.v[l]) tot->peak.v[l] = ramMb;
        if (testDefs[t].kind == kindCpu) tot->cpu.v[l] += c->internal;
        if (testDefs[t].kind == kindIo) tot->io.v[l] += c->internal;
        if (testDefs[t].kind != kindCpu && testDefs[t].kind != kindIo) continue;
        logSum += log(c->internal / bestInternal(t));
        scored++;
    }
    tot->score.has[l] = scored > 0;
    if (scored > 0) tot->score.v[l] = exp(logSum / scored);
}

static void printTotals(void) {
    Totals tot;
    memset(&tot, 0, sizeof tot);
    for (int l = 0; l < langCount; l++) {
        if (!langComplete(l)) continue;
        tot.wall.has[l] = tot.cpu.has[l] = tot.io.has[l] = tot.peak.has[l] = 1;
        addTotals(l, &tot);
    }
    printTitle("total");
    printRow("wall", &tot.wall, unitSec);
    printRow("cpu", &tot.cpu, unitSec);
    printRow("stdout", &tot.io, unitSec);
    printRow("peak ram", &tot.peak, unitMem);
    printRow("score", &tot.score, unitRatio);
}

static void printHeader(void) {
    printf("%-*s ", labelWidth, "");
    int printed = 0;
    for (int l = 0; l < langCount; l++) {
        if (langState[l] != langReady) continue;
        if (printed > 0 && strcmp(langs[l].name, "yukino") == 0) {
            printf("%*s ", cellWidth, "");
        }
        paint(langs[l].color);
        paint(styleBold);
        printf("%*s ", cellWidth, langs[l].name);
        paint(styleReset);
        printed++;
    }
    printf("\n");
}

static const char *getArtifactPath(int l) {
    if (strcmp(langs[l].name, "kotlin") == 0) return "build/kotlin/out.jar";
    return langs[l].runArgv[0];
}

static void printBuildAndStart(void) {
    Row build = {0};
    Row binSize = {0};
    for (int l = 0; l < langCount; l++) {
        build.has[l] = langState[l] == langReady;
        build.v[l] = buildSec[l];
        if (langState[l] == langReady) {
            struct stat st;
            const char *art = getArtifactPath(l);
            if (art != NULL && stat(art, &st) == 0) {
                binSize.has[l] = 1;
                binSize.v[l] = (double)st.st_size / 1024.0;
            }
        }
    }
    printRow("build", &build, unitSec);
    printRow("bin size", &binSize, unitSize);
    for (int t = 0; t < testCount; t++) {
        if (testDefs[t].kind != kindNop || !testSelected[t]) continue;
        Row start;
        fillRow(&start, t, metricWall);
        printRow("start", &start, unitSec);
        Row base;
        fillRow(&base, t, metricRss);
        printRow("base ram", &base, unitMem);
    }
}

static void printLine(const char *style, const char *text1, const char *text2) {
    if (useColor) fputs(style, stdout);
    printf("%s %s", text1, text2);
    if (useColor) fputs(styleReset, stdout);
    printf("\n");
}

static int printCellProblems(int t) {
    int bad = 0;
    char text[64];
    for (int l = 0; l < langCount; l++) {
        Status st = cells[t][l].status;
        if (st != statusFail && st != statusMismatch) continue;
        snprintf(text, sizeof text, "%s: %s", langs[l].name, st == statusFail ? "run failed" : "checksum differs");
        printLine(styleRed, testDefs[t].name, text);
        bad++;
    }
    return bad;
}

static void printNotes(void) {
    char text[PATH_MAX];
    for (int l = 0; l < langCount; l++) {
        if (langState[l] == langMissing) {
            snprintf(text, sizeof text, "%s: toolchain not found", langs[l].name);
            printLine(styleDim, "skip", text);
        }
        if (langState[l] == langBuildFailed) {
            snprintf(text, sizeof text, "%s: see build/%s/build.log", langs[l].name, langs[l].name);
            printLine(styleRed, "fail", text);
        }
    }
    int bad = 0;
    for (int t = 0; t < testCount; t++) bad += printCellProblems(t);
    if (bad == 0) printLine(styleGreen, "checksums", "match across languages");
}

typedef struct {
    int testIndex;
    int rank;
    char result[32];
} YukinoRank;

static int yukinoTestRank(int t, int ykn, Metric m, double yknVal) {
    int rank = 1;
    for (int i = 0; i < langCount; i++) {
        if (i == ykn || langState[i] != langReady) continue;
        const Cell *c = &cells[t][i];
        if (c->status != statusOk && c->status != statusMismatch) continue;
        if (metricOf(c, m) < yknVal - 1e-9) rank++;
    }
    return rank;
}

static int tableWidth(void) {
    int w = labelWidth + 1;
    int printed = 0;
    for (int i = 0; i < langCount; i++) {
        if (langState[i] != langReady) continue;
        if (printed > 0 && strcmp(langs[i].name, "yukino") == 0) w += cellWidth + 1;
        w += cellWidth + 1;
        printed++;
    }
    return w;
}

static void printDivider(void) {
    int w = tableWidth();
    paint(styleDim);
    for (int i = 0; i < w; i++) putchar('-');
    putchar('\n');
    paint(styleReset);
}

static int compareYukinoRank(const void *a, const void *b) {
    const YukinoRank *ra = (const YukinoRank *)a;
    const YukinoRank *rb = (const YukinoRank *)b;
    if (ra->rank != rb->rank) return ra->rank - rb->rank;
    return ra->testIndex - rb->testIndex;
}

static void printYukinoTop(void) {
    int ykn = -1;
    for (int i = 0; i < langCount; i++) {
        if (strcmp(langs[i].name, "yukino") == 0) {
            ykn = i;
            break;
        }
    }
    if (ykn < 0 || langState[ykn] != langReady) return;
    YukinoRank ranks[maxTests];
    int count = 0;
    for (int i = 0; i < testCount; i++) {
        if (!testSelected[i]) continue;
        const Cell *cykn = &cells[i][ykn];
        if (cykn->status == statusNone) continue;
        Kind kind = testDefs[i].kind;
        Metric m = kind == kindMem ? metricRss : (kind == kindNop ? metricWall : metricInternal);
        Unit u = kind == kindMem ? unitMem : unitSec;
        YukinoRank *r = &ranks[count++];
        r->testIndex = i;
        if (cykn->status == statusFail) {
            r->rank = maxLangs + 1;
            snprintf(r->result, sizeof r->result, "fail");
            continue;
        }
        double yknVal = metricOf(cykn, m);
        if (u == unitMem) snprintf(r->result, sizeof r->result, "%.1fM", yknVal);
        else snprintf(r->result, sizeof r->result, "%.2fs", yknVal);
        r->rank = yukinoTestRank(i, ykn, m, yknVal);
    }
    if (count == 0) return;
    qsort(ranks, count, sizeof(YukinoRank), compareYukinoRank);
    printDivider();
    printTitle("Yukino results (top)");
    for (int i = 0; i < count; i++) {
        const YukinoRank *r = &ranks[i];
        printf("%-*s ", labelWidth, testDefs[r->testIndex].name);
        paint(r->rank > langCount ? styleRed : langs[ykn].color);
        if (r->rank == 1) paint(styleBold);
        if (r->rank > langCount) printf("-. %s\n", r->result);
        else printf("%d. %s\n", r->rank, r->result);
        paint(styleReset);
    }
}

static int listHas(const char *list, const char *name) {
    size_t len = strlen(name);
    for (const char *p = list; *p != '\0';) {
        const char *end = strchr(p, ',');
        size_t part = end != NULL ? (size_t)(end - p) : strlen(p);
        if (part == len && strncmp(p, name, len) == 0) return 1;
        if (end == NULL) break;
        p = end + 1;
    }
    return 0;
}

static void usage(void) {
    puts("usage: speed_test [-n runs] [-l langs] [-t tests] [-v]");
    puts("  -n runs   repeats per test, best time and peak ram are kept (default 3)");
    puts("  -l langs  comma list, e.g. c,rust");
    puts("  -t tests  comma list, e.g. sieve,sort");
    puts("  -v        also print wall time and peak ram for every test");
}

int main(int argc, char **argv) {
    const char *langFilter = NULL;
    const char *testFilter = NULL;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-n") == 0 && i + 1 < argc) runs = atoi(argv[++i]);
        else if (strcmp(argv[i], "-l") == 0 && i + 1 < argc) langFilter = argv[++i];
        else if (strcmp(argv[i], "-t") == 0 && i + 1 < argc) testFilter = argv[++i];
        else if (strcmp(argv[i], "-v") == 0) verbose = 1;
        else {
            usage();
            return 1;
        }
    }
    if (runs < 1) runs = 1;
    useColor = (isatty(STDOUT_FILENO) || getenv("FORCE_COLOR") != NULL) && getenv("NO_COLOR") == NULL;
    enterRoot(argv[0]);
    system("rm -rf build yukino/yuki-work go/zzbuildid.go");
    for (int t = 0; t < testCount; t++) testSelected[t] = testFilter == NULL || listHas(testFilter, testDefs[t].name);
    for (int l = 0; l < langCount; l++) {
        if (langFilter == NULL || listHas(langFilter, langs[l].name)) buildLang(l);
    }
    for (int t = 0; t < testCount; t++) {
        if (!testSelected[t]) continue;
        for (int l = 0; l < langCount; l++) {
            if (langState[l] != langReady) continue;
            progress(testDefs[t].name, langs[l].name);
            benchCell(t, l);
        }
        markMismatches(t);
    }
    clearProgress();
    printHeader();
    printBuildAndStart();
    printSection("cpu", 1u << kindCpu, metricInternal, unitSec);
    printSection("stdout", 1u << kindIo, metricInternal, unitSec);
    printSection("mem", 1u << kindMem, metricRss, unitMem);
    printTotals();
    if (verbose) {
        printSection("wall", allKinds, metricWall, unitSec);
        printSection("ram", allKinds, metricRss, unitMem);
    }
    printNotes();
    printYukinoTop();
    return 0;
}
