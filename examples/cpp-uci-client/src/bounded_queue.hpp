// Bounded SPSC hand-off: the runtime has one dispatcher for all subscriptions.
// Only the consumer side takes a mutex. The producer never waits for space or
// for a lock, and does not allocate queue storage. Overflow is sticky.
#pragma once

#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <thread>
#include <utility>
#include <vector>

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

    explicit BoundedQueue(std::size_t capacity) : slots_(storage_size(capacity)) {}
    BoundedQueue(const BoundedQueue&) = delete;
    BoundedQueue& operator=(const BoundedQueue&) = delete;

    std::size_t capacity() const noexcept { return slots_.size() - 1; }

    Push push(T&& value) {
        std::size_t state = write_.load();
        if ((state & closed_bit) != 0) {
            ++rejected_closed_;
            return Push::closed;
        }
        const std::size_t tail = position(state);
        const std::size_t head = read_.load();
        const std::size_t next = advance(tail);
        const bool full = (state & overflow_bit) != 0 || next == head;
        if (!full) {
            slots_[tail].emplace(std::move(value));
        }
        // Only close() can change write_ concurrently. A slot staged before
        // close but not published belongs to this producer alone.
        if (!write_.compare_exchange_strong(state, full ? state | overflow_bit : next << flag_bits)) {
            if (!full) {
                slots_[tail].reset();
            }
            ++rejected_closed_;
            return Push::closed;
        }
        if (full) {
            ++rejected_full_;
            return Push::full;
        }
        ++accepted_;
        // Conservative peak based on the consumer position at insertion.
        const std::size_t used = distance(head, next);
        if (used > high_water_.load()) {
            high_water_.store(used);
        }
        return Push::accepted;
    }

    Pop pop_until(std::chrono::steady_clock::time_point deadline, T& out) {
        using Clock = std::chrono::steady_clock;
        for (;;) {
            {
                std::lock_guard<std::mutex> lock(consumer_);
                const std::size_t state = write_.load();
                if ((state & overflow_bit) != 0) {
                    return Pop::overflowed;
                }
                const std::size_t head = read_.load();
                if (head != position(state)) {
                    out = std::move(*slots_[head]);
                    slots_[head].reset();
                    read_.store(advance(head));
                    return Pop::item;
                }
                if ((state & closed_bit) != 0) {
                    return Pop::closed;
                }
            }
            const auto now = Clock::now();
            if (now >= deadline) {
                return Pop::timeout;
            }
            // C++17 has no atomic wait. Poll only on the application thread;
            // condition-variable notification without a producer lock can miss
            // the transition from an empty queue to a waiting consumer.
            const auto slice = now + std::chrono::milliseconds(1);
            std::this_thread::sleep_until(slice < deadline ? slice : deadline);
        }
    }

    std::size_t close() {
        std::lock_guard<std::mutex> lock(consumer_);
        const std::size_t state = write_.fetch_or(closed_bit);
        if ((state & closed_bit) != 0) {
            return 0;
        }
        const std::size_t tail = position(state);
        const std::size_t head = read_.load();
        for (std::size_t i = head; i != tail; i = advance(i)) {
            slots_[i].reset();
        }
        read_.store(tail);
        discarded_ = distance(head, tail);
        return discarded_;
    }

    std::size_t size() const {
        std::lock_guard<std::mutex> lock(consumer_);
        return distance(read_.load(), position(write_.load()));
    }
    bool overflowed() const noexcept { return (write_.load() & overflow_bit) != 0; }
    Counters counters() const {
        std::lock_guard<std::mutex> lock(consumer_);
        return {accepted_.load(), rejected_full_.load(), rejected_closed_.load(), discarded_, high_water_.load()};
    }

private:
    static_assert(std::atomic<std::size_t>::is_always_lock_free && std::atomic<std::uint64_t>::is_always_lock_free,
                  "SPSC producer counters and cursors must be lock-free");
    static constexpr unsigned flag_bits = 2;
    static constexpr std::size_t closed_bit = 1;
    static constexpr std::size_t overflow_bit = 2;

    static std::size_t storage_size(std::size_t capacity) {
        if (capacity > (std::numeric_limits<std::size_t>::max() >> flag_bits)) {
            throw std::length_error("application queue capacity exceeds cursor range");
        }
        return (capacity == 0 ? 1 : capacity) + 1;
    }

    static std::size_t position(std::size_t state) noexcept { return state >> flag_bits; }
    std::size_t advance(std::size_t i) const noexcept { return i + 1 == slots_.size() ? 0 : i + 1; }
    std::size_t distance(std::size_t head, std::size_t tail) const noexcept {
        return head <= tail ? tail - head : slots_.size() - head + tail;
    }

    std::vector<std::optional<T>> slots_; // one sentinel slot distinguishes full from empty
    mutable std::mutex consumer_;         // only pop, observation and close; never push
    std::atomic<std::size_t> read_{0};
    std::atomic<std::size_t> write_{0};    // published tail and sticky flags
    std::atomic<std::uint64_t> accepted_{0}, rejected_full_{0}, rejected_closed_{0};
    std::atomic<std::size_t> high_water_{0};
    std::size_t discarded_ = 0;
};

}  // namespace cpp_uci_client