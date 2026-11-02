// CPP-OMS-003 TEST-ONLY controlled OWP peer (Boost.Beast server on 127.0.0.1).
//
// Not a broker and not shipped: it records every client frame and replies
// exactly what each test scripts. It enforces the `owp` subprotocol with HTTP
// 400 by default, and can be told to misbehave (no/wrong subprotocol, accept
// TCP but never upgrade, ignore commands, drop the TCP connection, send
// binary or malformed frames) so the runtime's failure paths are exercised.
#pragma once

#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/post.hpp>
#include <boost/asio/steady_timer.hpp>
#include <boost/beast/core.hpp>
#include <boost/beast/http.hpp>
#include <boost/beast/websocket.hpp>

#include <chrono>
#include <condition_variable>
#include <deque>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

namespace mock {

namespace net = boost::asio;
namespace beast = boost::beast;
namespace http = beast::http;
namespace websocket = beast::websocket;
using tcp = net::ip::tcp;

struct Action {
    enum class Kind { text, binary, close, drop, delay };
    Kind kind = Kind::text;
    std::string data;
    std::chrono::milliseconds delay{0};

    static Action text(std::string frame) { return Action{Kind::text, std::move(frame), {}}; }
    static Action binary(std::string bytes) { return Action{Kind::binary, std::move(bytes), {}}; }
    static Action close() { return Action{Kind::close, {}, {}}; }
    static Action drop() { return Action{Kind::drop, {}, {}}; }
    static Action wait(std::chrono::milliseconds ms) { return Action{Kind::delay, {}, ms}; }
};

using Responder = std::function<std::vector<Action>(const std::string& frame)>;

inline const std::string kInfo =
    R"(INFO {"version":"1.0","server_id":"mock-peer","uuids":{"system":"5d0f7f53-8bb4-4bd3-9a84-6c0e5bb9d001","service":"5d0f7f53-8bb4-4bd3-9a84-6c0e5bb9d002"},"system_label":"CPP-OMS-003 mock"})";

/// Default script: INIT -> +OK, INFO; any other operation -> +OK.
inline std::vector<Action> acknowledge_everything(const std::string& frame) {
    if (frame.rfind("INIT ", 0) == 0) {
        return {Action::text("+OK"), Action::text(kInfo)};
    }
    return {Action::text("+OK")};
}

class Peer {
public:
    enum class Handshake { owp, reject_400, omit_subprotocol, wrong_subprotocol, never_upgrade };

    explicit Peer(Responder responder = acknowledge_everything, Handshake handshake = Handshake::owp)
        : responder_(std::move(responder)), handshake_(handshake), acceptor_(ioc_) {
        tcp::endpoint endpoint(net::ip::make_address("127.0.0.1"), 0);
        acceptor_.open(endpoint.protocol());
        acceptor_.set_option(net::socket_base::reuse_address(true));
        acceptor_.bind(endpoint);
        acceptor_.listen();
        port_ = acceptor_.local_endpoint().port();
        accept();
        thread_ = std::thread([this] { ioc_.run(); });
    }

    Peer(const Peer&) = delete;
    Peer& operator=(const Peer&) = delete;

    ~Peer() {
        net::post(ioc_, [this] {
            beast::error_code ignored;
            acceptor_.close(ignored);
            for (auto& socket : raw_) {
                socket->close(ignored);
            }
            if (session_) {
                beast::get_lowest_layer(session_->ws).close();
            }
            ioc_.stop();
        });
        thread_.join();
    }

    std::string url() const { return "ws://127.0.0.1:" + std::to_string(port_); }
    unsigned short port() const { return port_; }

    /// Text frames received from the client, in order.
    std::vector<std::string> frames() {
        std::lock_guard<std::mutex> lock(mutex_);
        return frames_;
    }

    bool wait_frames(std::size_t count, std::chrono::milliseconds timeout = std::chrono::seconds(5)) {
        std::unique_lock<std::mutex> lock(mutex_);
        return cv_.wait_for(lock, timeout, [&] { return frames_.size() >= count; });
    }

    bool wait_disconnect(std::chrono::milliseconds timeout = std::chrono::seconds(5)) {
        std::unique_lock<std::mutex> lock(mutex_);
        return cv_.wait_for(lock, timeout, [&] { return disconnected_; });
    }

    bool client_sent_close() {
        std::lock_guard<std::mutex> lock(mutex_);
        return client_close_;
    }

    std::string requested_subprotocol() {
        std::lock_guard<std::mutex> lock(mutex_);
        return requested_subprotocol_;
    }

    int connections() {
        std::lock_guard<std::mutex> lock(mutex_);
        return connections_;
    }

    /// Sends actions on the current connection from the test thread.
    void inject(std::vector<Action> actions) {
        net::post(ioc_, [this, actions = std::move(actions)]() mutable {
            if (session_) {
                enqueue(session_, std::move(actions));
            }
        });
    }

    void set_responder(Responder responder) {
        std::lock_guard<std::mutex> lock(mutex_);
        responder_ = std::move(responder);
    }

private:
    struct Session {
        explicit Session(tcp::socket socket) : ws(std::move(socket)) {}
        websocket::stream<beast::tcp_stream> ws;
        beast::flat_buffer buffer;
        http::request<http::string_body> request;
        std::deque<Action> actions;
        bool busy = false;
        bool dead = false;
        std::unique_ptr<net::steady_timer> timer;
    };

    void accept() {
        acceptor_.async_accept([this](beast::error_code ec, tcp::socket socket) {
            if (ec) {
                return;
            }
            beast::error_code ignored;
            socket.set_option(tcp::no_delay(true), ignored);
            {
                std::lock_guard<std::mutex> lock(mutex_);
                ++connections_;
                disconnected_ = false;
            }
            if (handshake_ == Handshake::never_upgrade) {
                raw_.push_back(std::make_shared<tcp::socket>(std::move(socket)));
            } else {
                auto session = std::make_shared<Session>(std::move(socket));
                session_ = session;
                read_upgrade(session);
            }
            accept();
        });
    }

    void read_upgrade(const std::shared_ptr<Session>& session) {
        http::async_read(beast::get_lowest_layer(session->ws), session->buffer, session->request,
                         [this, session](beast::error_code ec, std::size_t) {
                             if (ec) {
                                 return finish(session);
                             }
                             {
                                 std::lock_guard<std::mutex> lock(mutex_);
                                 requested_subprotocol_ =
                                     std::string(session->request[http::field::sec_websocket_protocol]);
                             }
                             const bool offered = session->request[http::field::sec_websocket_protocol] == "owp";
                             if (handshake_ == Handshake::reject_400 || !offered) {
                                 auto response = std::make_shared<http::response<http::string_body>>(
                                     http::status::bad_request, session->request.version());
                                 response->body() = "missing owp subprotocol";
                                 response->prepare_payload();
                                 http::async_write(beast::get_lowest_layer(session->ws), *response,
                                                   [this, session, response](beast::error_code, std::size_t) {
                                                       beast::get_lowest_layer(session->ws).close();
                                                       finish(session);
                                                   });
                                 return;
                             }
                             const Handshake mode = handshake_;
                             session->ws.set_option(websocket::stream_base::decorator(
                                 [mode](websocket::response_type& response) {
                                     if (mode == Handshake::owp) {
                                         response.set(http::field::sec_websocket_protocol, "owp");
                                     } else if (mode == Handshake::wrong_subprotocol) {
                                         response.set(http::field::sec_websocket_protocol, "not-owp");
                                     }
                                 }));
                             session->ws.async_accept(session->request, [this, session](beast::error_code accept_ec) {
                                 if (accept_ec) {
                                     return finish(session);
                                 }
                                 session->buffer.consume(session->buffer.size());
                                 read(session);
                             });
                         });
    }

    void read(const std::shared_ptr<Session>& session) {
        session->ws.async_read(session->buffer, [this, session](beast::error_code ec, std::size_t) {
            if (ec) {
                if (ec == websocket::error::closed) {
                    std::lock_guard<std::mutex> lock(mutex_);
                    client_close_ = true;
                }
                return finish(session);
            }
            std::string frame = beast::buffers_to_string(session->buffer.data());
            session->buffer.consume(session->buffer.size());
            Responder responder;
            {
                std::lock_guard<std::mutex> lock(mutex_);
                frames_.push_back(frame);
                responder = responder_;
            }
            cv_.notify_all();
            enqueue(session, responder(frame));
            read(session);
        });
    }

    void enqueue(const std::shared_ptr<Session>& session, std::vector<Action> actions) {
        for (auto& action : actions) {
            session->actions.push_back(std::move(action));
        }
        if (!session->busy) {
            run(session);
        }
    }

    void run(const std::shared_ptr<Session>& session) {
        if (session->dead || session->actions.empty()) {
            session->busy = false;
            return;
        }
        session->busy = true;
        Action action = std::move(session->actions.front());
        session->actions.pop_front();
        auto next = [this, session](beast::error_code ec, std::size_t = 0) {
            if (ec) {
                session->dead = true;
                session->busy = false;
                return;
            }
            run(session);
        };
        switch (action.kind) {
            case Action::Kind::text:
            case Action::Kind::binary: {
                auto data = std::make_shared<std::string>(std::move(action.data));
                session->ws.text(action.kind == Action::Kind::text);
                session->ws.async_write(net::buffer(*data), [next, data](beast::error_code ec, std::size_t n) { next(ec, n); });
                return;
            }
            case Action::Kind::close:
                session->ws.async_close(websocket::close_code::normal, [next](beast::error_code ec) { next(ec); });
                return;
            case Action::Kind::drop: {
                beast::error_code ignored;
                beast::get_lowest_layer(session->ws).socket().shutdown(tcp::socket::shutdown_both, ignored);
                beast::get_lowest_layer(session->ws).close();
                session->dead = true;
                session->busy = false;
                return;
            }
            case Action::Kind::delay:
                session->timer = std::make_unique<net::steady_timer>(ioc_, action.delay);
                session->timer->async_wait([next](beast::error_code ec) { next(ec); });
                return;
        }
    }

    void finish(const std::shared_ptr<Session>& session) {
        session->dead = true;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            disconnected_ = true;
        }
        cv_.notify_all();
    }

    Responder responder_;
    const Handshake handshake_;
    net::io_context ioc_;
    tcp::acceptor acceptor_;
    unsigned short port_ = 0;
    std::thread thread_;
    std::shared_ptr<Session> session_;
    std::vector<std::shared_ptr<tcp::socket>> raw_;

    std::mutex mutex_;
    std::condition_variable cv_;
    std::vector<std::string> frames_;
    std::string requested_subprotocol_;
    int connections_ = 0;
    bool disconnected_ = false;
    bool client_close_ = false;
};

}  // namespace mock
