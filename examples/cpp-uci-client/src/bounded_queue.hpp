// CPP-OMS-008: explicitly bounded, ordered hand-off from the runtime's
// dispatcher thread (typed callback) to the application's main thread.
//
// push() never blocks, so a typed callback can never stall the runtime
// dispatcher or an unsubscribe() waiting for it. A full queue is NOT a
// silent drop: the queue becomes `overflowed` (sticky) and the consumer is
// woken to fail the session explicitly. After close() every push is refused
// and counted. Only the queue's own mutex is held, and never while the
// caller invokes a runtime operation.
#pragma once

#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <mutex>
#include <utility>

namespace cpp_uci_client {

template <typename T>
class BoundedQueue {
public:
    enum class Push { accepted, full, closed };
    enum class Pop { item, timeout, overflowed, closed };

    struct Counters {
        std::uint64_t accepted = 0;
        std::uint64_t rejected_full = 0;
        std::uint64_t rejected_closed = 0;
        std::uint64_t discarded_on_close = 0;
        std::size_t high_water = 0;
    };

    /// `capacity` must be at least 1 (0 is treated as 1).
    explicit BoundedQueue(std::size_t capacity) : capacity_(capacity == 0 ? 1 : capacity) {}
    BoundedQueue(const BoundedQueue&) = delete;
    BoundedQueue& operator=(const BoundedQueue&) = delete;

    std::size_t capacity() const noexcept { return capacity_; }

    /// Producer side; never blocks beyond the queue mutex.
    Push push(T&& value) {
        Push result = Push::accepted;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (closed_) {
                ++counters_.rejected_closed;
                result = Push::closed;
            } else if (overflowed_ || items_.size() >= capacity_) {
                overflowed_ = true;
                ++counters_.rejected_full;
                result = Push::full;
            } else {
                items_.push_back(std::move(value));
                ++counters_.accepted;
                if (items_.size() > counters_.high_water) {
                    counters_.high_water = items_.size();
                }
            }
        }
        if (result != Push::closed) {
            cv_.notify_all();
        }
        return result;
    }

    /// Consumer side. Overflow is reported before any further item (fail
    /// fast: the in-order sequence already has a gap). Items are FIFO.
    Pop pop_until(std::chrono::steady_clock::time_point deadline, T& out) {
        std::unique_lock<std::mutex> lock(mutex_);
        cv_.wait_until(lock, deadline, [this] { return overflowed_ || closed_ || !items_.empty(); });
        if (overflowed_) {
            return Pop::overflowed;
        }
        if (!items_.empty()) {
            out = std::move(items_.front());
            items_.pop_front();
            return Pop::item;
        }
        return closed_ ? Pop::closed : Pop::timeout;
    }

    /// Refuses later pushes; discards and returns the count of pending items.
    /// Idempotent (a second call returns 0).
    std::size_t close() {
        std::size_t discarded = 0;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (closed_) {
                return 0;
            }
            closed_ = true;
            discarded = items_.size();
            counters_.discarded_on_close += discarded;
            items_.clear();
        }
        cv_.notify_all();
        return discarded;
    }

    std::size_t size() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return items_.size();
    }

    bool overflowed() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return overflowed_;
    }

    Counters counters() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return counters_;
    }

private:
    const std::size_t capacity_;
    mutable std::mutex mutex_;
    std::condition_variable cv_;
    std::deque<T> items_;
    bool closed_ = false;
    bool overflowed_ = false;
    Counters counters_;
};

}  // namespace cpp_uci_client
