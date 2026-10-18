/* Test-only NULL injector isolation. No provider/runtime exception inference. */
#include <stddef.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
extern void task097_c1_fail_next_malloc(void);
extern size_t task097_c1_malloc_failures(void);
extern size_t task097_c1_malloc_size(void);
static atomic_int ready, start, done;
static int worker_ok;
static void *worker(void *unused) {
    (void)unused;
    atomic_store(&ready, 1);
    while (!atomic_load(&start)) { }
    /* The other thread is armed. This call must neither fail nor consume
       that thread's budget. This worker then exercises its independent one. */
    void *pointer = malloc(19);
    worker_ok = pointer != NULL && task097_c1_malloc_failures() == 0;
    free(pointer);
    task097_c1_fail_next_malloc();
    pointer = malloc(23);
    worker_ok = worker_ok && pointer == NULL &&
        task097_c1_malloc_failures() == 1 && task097_c1_malloc_size() == 23;
    pointer = malloc(29);
    worker_ok = worker_ok && pointer != NULL && task097_c1_malloc_failures() == 1;
    free(pointer);
    atomic_store(&done, 1);
    return NULL;
}
int main(void) {
    pthread_t thread;
    if (pthread_create(&thread, NULL, worker, NULL)) return 1;
    while (!atomic_load(&ready)) { }
    const size_t before = task097_c1_malloc_failures();
    task097_c1_fail_next_malloc();
    atomic_store(&start, 1);
    while (!atomic_load(&done)) { }
    void *pointer = malloc(31);
    int valid = pointer == NULL && task097_c1_malloc_failures() == before + 1 &&
        task097_c1_malloc_size() == 31;
    pointer = malloc(37);
    valid = valid && pointer != NULL && task097_c1_malloc_failures() == before + 1;
    /* realloc failure must not invalidate the original allocation. */
    if (pointer) {
        ((char *)pointer)[0] = 'X';
        task097_c1_fail_next_malloc();
        void *replacement = realloc(pointer, 41);
        valid = valid && replacement == NULL && ((char *)pointer)[0] == 'X' &&
            task097_c1_malloc_size() == 41 && task097_c1_malloc_failures() == before + 2;
        free(pointer);
    }
    if (pthread_join(thread, NULL) || !valid || !worker_ok) return 2;
    puts("TASK097_C1_ALLOCATOR_ISOLATION_COMPLETE threads=2 one_shot=true realloc_preserved=true");
    return 0;
}