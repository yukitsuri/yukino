#include <stdlib.h>
#include <time.h>

#define MAX_SNAKE_LEN 1024

typedef struct {
    int x;
    int y;
} SnakePoint;

static SnakePoint g_snake[MAX_SNAKE_LEN];
static int g_snake_len = 0;
static SnakePoint g_last_tail = {0, 0};

void snake_init(int start_x, int start_y, int initial_len) {
    if (initial_len < 1) initial_len = 1;
    if (initial_len > MAX_SNAKE_LEN) initial_len = MAX_SNAKE_LEN;

    g_snake_len = initial_len;
    for (int i = 0; i < initial_len; i++) {
        g_snake[i].x = start_x - i;
        g_snake[i].y = start_y;
    }
    g_last_tail = g_snake[initial_len - 1];
    srand((unsigned int)time(NULL));
}

int snake_get_length(void) {
    return g_snake_len;
}

int snake_get_head_x(void) {
    return g_snake_len > 0 ? g_snake[0].x : 0;
}

int snake_get_head_y(void) {
    return g_snake_len > 0 ? g_snake[0].y : 0;
}

int snake_get_tail_x(void) {
    return g_last_tail.x;
}

int snake_get_tail_y(void) {
    return g_last_tail.y;
}

int snake_contains(int x, int y, int skip_head) {
    int start = skip_head ? 1 : 0;
    for (int i = start; i < g_snake_len; i++) {
        if (g_snake[i].x == x && g_snake[i].y == y) {
            return 1;
        }
    }
    return 0;
}

int snake_step(int new_x, int new_y, int grow) {
    if (snake_contains(new_x, new_y, 0)) {
        return 1;
    }

    if (grow) {
        if (g_snake_len < MAX_SNAKE_LEN) {
            for (int i = g_snake_len; i > 0; i--) {
                g_snake[i] = g_snake[i - 1];
            }
            g_snake[0].x = new_x;
            g_snake[0].y = new_y;
            g_snake_len++;
        }
    } else {
        g_last_tail = g_snake[g_snake_len - 1];
        for (int i = g_snake_len - 1; i > 0; i--) {
            g_snake[i] = g_snake[i - 1];
        }
        g_snake[0].x = new_x;
        g_snake[0].y = new_y;
    }

    return 0;
}

int snake_rand_range(int min_val, int max_val) {
    if (min_val >= max_val) return min_val;
    return min_val + (rand() % (max_val - min_val + 1));
}
