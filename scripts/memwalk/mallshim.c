// docs/architecture/memory.md, "A finished race held under its replay (ADR-0099 step 5)".
// LD_PRELOAD shim: SIGUSR1 appends mallinfo2 to $MALLSHIM_OUT, SIGUSR2 calls
// malloc_trim(0) first. A thread in sigwait does the work, so nothing runs in a
// signal handler.
#define _GNU_SOURCE
#include <malloc.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>

static void *serve(void *arg) {
    sigset_t set;
    sigemptyset(&set);
    sigaddset(&set, SIGUSR1);
    sigaddset(&set, SIGUSR2);
    const char *out = getenv("MALLSHIM_OUT");
    unsigned long n = 0;
    for (;;) {
        int sig = 0;
        if (sigwait(&set, &sig) != 0) continue;
        if (sig == SIGUSR2) malloc_trim(0);
        struct mallinfo2 m = mallinfo2();
        FILE *f = fopen(out, "a");
        if (!f) continue;
        fprintf(f, "{\"n\":%lu,\"trim\":%d,\"arena\":%zu,\"hblkhd\":%zu,\"uordblks\":%zu,\"fordblks\":%zu}\n",
                ++n, sig == SIGUSR2, m.arena, m.hblkhd, m.uordblks, m.fordblks);
        fclose(f);
    }
    return arg;
}

__attribute__((constructor)) static void start(void) {
    if (!getenv("MALLSHIM_OUT")) return;
    sigset_t set;
    sigemptyset(&set);
    sigaddset(&set, SIGUSR1);
    sigaddset(&set, SIGUSR2);
    pthread_sigmask(SIG_BLOCK, &set, NULL);
    pthread_t t;
    pthread_create(&t, NULL, serve, NULL);
    pthread_detach(t);
}
