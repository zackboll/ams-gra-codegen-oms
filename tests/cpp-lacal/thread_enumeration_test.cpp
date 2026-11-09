// No broker or runtime: distinguish joined Linux task records from live workers.
#include "check.hpp"
#include <pthread.h>
#include <condition_variable>
#include <mutex>

TEST_CASE(stat_parser_fails_closed) {
    CHECK(!check::task_flags("").has_value());
    CHECK(!check::task_flags("123 (name) R 1").has_value());
    CHECK(check::task_flags("123 (name with ) spaces) R 1 2 3 4 5 4194380 0").value() == 4194380UL);
    CHECK(check::task_flags("123 (name) S 1 2 3 4 5 0 0").value() == 0UL);
}

TEST_CASE(live_worker_is_not_hidden) {
    std::mutex mutex;
    std::condition_variable cv;
    bool stop = false;
    std::thread worker([&] {
        std::unique_lock<std::mutex> lock(mutex);
        cv.wait(lock, [&] { return stop; });
    });
    CHECK(pthread_setname_np(worker.native_handle(), "ams-probe-live") == 0);
    CHECK(check::threads_named("ams-probe-live") == 1);
    {
        std::lock_guard<std::mutex> lock(mutex);
        stop = true;
    }
    cv.notify_one();
    worker.join();
    CHECK(check::threads_named("ams-probe-live") == 0);
}

TEST_CASE(joined_worker_is_not_a_live_task) {
    for (int i = 0; i < 2000; ++i) {
        std::atomic<bool> release{false};
        std::atomic<bool> complete{false};
        std::thread worker([&] {
            while (!release.load()) { std::this_thread::yield(); }
            complete.store(true);
        });
        CHECK(pthread_setname_np(worker.native_handle(), "ams-probe-join") == 0);
        release.store(true);
        worker.join();
        CHECK(complete.load());
        CHECK(check::threads_named("ams-probe-join") == 0);
    }
}

int main(int argc, char** argv) {
    return check::run_all("CPP_LIVE_THREAD_ENUMERATION", argc, argv);
}