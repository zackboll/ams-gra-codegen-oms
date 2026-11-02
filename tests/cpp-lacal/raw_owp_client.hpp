// CPP-OMS-003 TEST-ONLY independent OWP client for real-broker evidence.
//
// Deliberately shares NO code with cpp/oms-lacal-runtime: its own handshake,
// INIT, verbose status handling and frame strings, written directly on
// Boost.Beast. It is used (a) as an observer that records the exact MSG text
// the broker delivers, and (b) as a publisher of hand-written malformed
// documents, so broker behaviour is observed independently of the runtime
// and of the generated codec. Single-threaded: every call runs its own
// io_context for a bounded time.
#pragma once

#include <boost/asio/ip/tcp.hpp>
#include <boost/beast/core.hpp>
#include <boost/beast/websocket.hpp>

#include <chrono>
#include <optional>
#include <stdexcept>
#include <string>

namespace raw {

namespace net = boost::asio;
namespace beast = boost::beast;
namespace websocket = beast::websocket;
using tcp = net::ip::tcp;

class Client {
public:
    Client(const std::string& host, const std::string& port, const std::string& service_id,
           const std::string& schema, std::chrono::milliseconds timeout)
        : ws_(ioc_), timeout_(timeout) {
        tcp::resolver resolver(ioc_);
        auto const results = resolver.resolve(host, port);
        beast::get_lowest_layer(ws_).connect(results);
        ws_.set_option(websocket::stream_base::decorator([](websocket::request_type& request) {
            request.set(beast::http::field::sec_websocket_protocol, "owp");
        }));
        websocket::response_type response;
        ws_.handshake(response, host + ":" + port, "/");
        if (response[beast::http::field::sec_websocket_protocol] != "owp") {
            throw std::runtime_error("raw client: subprotocol not negotiated");
        }
        ws_.text(true);
        send("INIT {\"versions\":[\"1.0\"],\"schema\":\"" + schema + "\",\"verbose\":true,\"service_id\":\"" +
             service_id + "\"}");
        if (receive().value_or("") != "+OK") {
            throw std::runtime_error("raw client: INIT not acknowledged");
        }
        if (receive().value_or("").rfind("INFO ", 0) != 0) {
            throw std::runtime_error("raw client: no INFO");
        }
    }

    ~Client() {
        beast::error_code ignored;
        beast::get_lowest_layer(ws_).socket().close(ignored);
    }

    void send(const std::string& frame) { ws_.write(net::buffer(frame)); }

    /// Next text frame, or nullopt if none arrives within `wait` (the read
    /// stays pending and is resumed by the next call).
    std::optional<std::string> receive(std::optional<std::chrono::milliseconds> wait = std::nullopt) {
        if (!reading_) {
            reading_ = true;
            done_ = false;
            ws_.async_read(buffer_, [this](beast::error_code ec, std::size_t) {
                error_ = ec;
                done_ = true;
            });
        }
        ioc_.restart();
        ioc_.run_for(wait.value_or(timeout_));
        if (!done_) {
            return std::nullopt;
        }
        reading_ = false;
        if (error_) {
            throw std::runtime_error("raw client read failed: " + error_.message());
        }
        std::string frame = beast::buffers_to_string(buffer_.data());
        buffer_.consume(buffer_.size());
        return frame;
    }

    /// Sends one command and returns its verbose status frame ("+OK" or "-ERR ...").
    std::string command(const std::string& frame) {
        send(frame);
        auto status = receive();
        if (!status) {
            throw std::runtime_error("raw client: no status for " + frame.substr(0, 40));
        }
        return *status;
    }

private:
    net::io_context ioc_;
    websocket::stream<beast::tcp_stream> ws_;
    std::chrono::milliseconds timeout_;
    beast::flat_buffer buffer_;
    bool reading_ = false;
    bool done_ = false;
    beast::error_code error_;
};

}  // namespace raw
