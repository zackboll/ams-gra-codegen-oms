"""Bounded, fail-closed client framing for the opt-in pre-broker relay.

No bytes of a WebSocket frame are released until its complete boundary and
payload have been validated. HTTP upgrade bytes and frame bytes are unchanged.
"""
MAX_FRAME = 1_048_576  # OMS_LACAL.Max_Frame_Bytes
MAX_UPGRADE = 16_384


class Parser:
    def __init__(self):
        self.buffer = bytearray()
        self.upgraded = False
        self.offset = 0

    def feed(self, data):
        # A caller supplies bounded TLS reads, not an unbounded corpus.
        if len(data) > 65536:
            raise ValueError("oversized input chunk")
        self.buffer.extend(data)
        result = []
        if not self.upgraded:
            end = self.buffer.find(b"\r\n\r\n")
            if end < 0:
                if len(self.buffer) > MAX_UPGRADE:
                    raise ValueError("oversized upgrade")
                return result
            end += 4
            if end > MAX_UPGRADE:
                raise ValueError("oversized upgrade")
            raw = bytes(self.buffer[:end])
            if not raw.startswith(b"GET ") or b"sec-websocket-protocol: owp" not in raw.lower():
                raise ValueError("unexpected client upgrade")
            del self.buffer[:end]
            result.append((self.offset, raw, None))
            self.offset += end
            self.upgraded = True
        while len(self.buffer) >= 2:
            first, second = self.buffer[:2]
            opcode, size = first & 15, second & 127
            if first & 0xF0 != 0x80 or opcode not in (1, 8, 9, 10):
                raise ValueError("fragmentation, reserved bits or unsupported opcode")
            if not second & 128:
                raise ValueError("unmasked client frame")
            offset = 2
            if size in (126, 127):
                width = 2 if size == 126 else 8
                if len(self.buffer) < 2 + width:
                    break
                size = int.from_bytes(self.buffer[2:2 + width], "big")
                if size < (126 if width == 2 else 65536):
                    raise ValueError("noncanonical length")
                offset += width
            if size > MAX_FRAME or (opcode >= 8 and size > 125):
                raise ValueError("oversized frame")
            end = offset + 4 + size
            if len(self.buffer) < end:
                break
            raw = bytes(self.buffer[:end])
            mask = raw[offset:offset + 4]
            payload = bytes(v ^ mask[i % 4] for i, v in enumerate(raw[offset + 4:]))
            text = payload.decode("utf-8") if opcode == 1 else None
            if opcode == 8 and size == 1:
                raise ValueError("malformed Close")
            del self.buffer[:end]
            result.append((self.offset, raw, text))
            self.offset += end
        if len(self.buffer) > MAX_FRAME + 14:
            raise ValueError("unbounded frame buffer")
        return result

    def finish(self):
        if not self.upgraded or self.buffer:
            raise ValueError("incomplete upgrade or WebSocket frame")


class Gate:
    def __init__(self, mode):
        self.mode = mode
        self.parser = Parser()
        self.subs = 0
        self.target = None

    def feed(self, data):
        forwarded = bytearray()
        for offset, raw, text in self.parser.feed(data):
            if self.target is not None:
                raise ValueError("unexpected frame after intercepted command")
            if text and text.startswith("SUB "):
                self.subs += 1
            selected = ((self.mode == "sub" and text and text.startswith("SUB ") and self.subs == 3)
                        or (self.mode == "unsub" and text == "UNSUB 3"))
            if selected:
                self.target = (offset, raw, text)
            else:
                forwarded.extend(raw)
        return bytes(forwarded)