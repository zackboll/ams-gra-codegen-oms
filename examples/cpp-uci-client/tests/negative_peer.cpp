// CPP-OMS-008 TEST-ONLY scripted loopback peer for failure paths that the
// unmodified Sleet broker cannot be made to exhibit. Built on the existing
// CPP-OMS-003 controlled peer (tests/cpp-lacal/mock_peer.hpp); it is not a
// broker and is never used for positive interoperability evidence.
//
//   negative_peer MODE SECONDS
//     prints "PORT <n>" and serves MODE for SECONDS, then prints the client
//     frames it received ("FRAME <text>", truncated) and exits.
//
// MODE:
//   no-owp          101 upgrade without Sec-WebSocket-Protocol
//   wrong-owp       101 upgrade selecting "not-owp"
//   reject-400      HTTP 400 instead of an upgrade
//   never-upgrade   accept TCP, never answer the upgrade (connect timeout)
//   silent-sub      INIT -> +OK + INFO; SUB is never acknowledged (operation timeout)
//   drop-after-sub  INIT -> +OK + INFO; SUB -> +OK, then the TCP connection is dropped
#include <chrono>
#include <cstdlib>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

#include "mock_peer.hpp"

int main(int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: negative_peer MODE SECONDS\n";
        return 2;
    }
    const std::string mode = argv[1];
    const int seconds = std::atoi(argv[2]);
    using mock::Action;
    using Handshake = mock::Peer::Handshake;
    Handshake handshake = Handshake::owp;
    mock::Responder responder = mock::acknowledge_everything;
    if (mode == "no-owp") {
        handshake = Handshake::omit_subprotocol;
    } else if (mode == "wrong-owp") {
        handshake = Handshake::wrong_subprotocol;
    } else if (mode == "reject-400") {
        handshake = Handshake::reject_400;
    } else if (mode == "never-upgrade") {
        handshake = Handshake::never_upgrade;
    } else if (mode == "silent-sub") {
        responder = [](const std::string& frame) -> std::vector<Action> {
            if (frame.rfind("INIT ", 0) == 0) {
                return {Action::text("+OK"), Action::text(mock::kInfo)};
            }
            return {};  // never acknowledge SUB
        };
    } else if (mode == "drop-after-sub") {
        responder = [](const std::string& frame) -> std::vector<Action> {
            if (frame.rfind("INIT ", 0) == 0) {
                return {Action::text("+OK"), Action::text(mock::kInfo)};
            }
            if (frame.rfind("SUB ", 0) == 0) {
                return {Action::text("+OK"), Action::wait(std::chrono::milliseconds(300)), Action::drop()};
            }
            return {Action::text("+OK")};
        };
    } else {
        std::cerr << "unknown mode " << mode << '\n';
        return 2;
    }
    mock::Peer peer(responder, handshake);
    std::cout << "PORT " << peer.port() << std::endl;
    std::this_thread::sleep_for(std::chrono::seconds(seconds));
    std::cout << "REQUESTED-SUBPROTOCOL " << peer.requested_subprotocol() << std::endl;
    for (const std::string& frame : peer.frames()) {
        std::cout << "FRAME " << frame.substr(0, 160) << std::endl;
    }
    std::cout << "CONNECTIONS " << peer.connections() << std::endl;
    return 0;
}
