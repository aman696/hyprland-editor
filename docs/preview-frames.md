# Sealed raw frame boundary

`crates/editor-preview/src/frame.rs` implements the first broker-to-editor pixel boundary. It does not capture a compositor output, connect a Wayland socket, route input or certify preview readiness. The private broker channel, descriptor queue limits, peer lifecycle and graphical display integration remain required.

## Wire record

One descriptor contains a 64-byte header followed by tightly packed, straight-alpha RGBA8888 pixels with sRGB encoded channels. Multi-byte header integers are little-endian. There is no compressed image or decoder.

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 0 | 8 | Fixed `HYEFRA01` protocol magic/version |
| 8 | 16 | Live session correlation token |
| 24 | 8 | Nonzero projection generation |
| 32 | 8 | Nonzero, strictly increasing frame sequence |
| 40 | 4 | Width |
| 44 | 4 | Height |
| 48 | 4 | Exact stride: width × 4 |
| 52 | 4 | Format identifier: 1 |
| 56 | 8 | Exact payload byte length |

The initial profile permits either orientation up to 1280 pixels on either axis and at most 1280 × 720 pixels total. Zero dimensions, larger profiles, padding, alternative formats, extra bytes and truncated records are refused. Every header field is checked against the fixed profile and current receiver context. A new projection constructs a new receiver. The session token prevents accidental cross-session reuse; it is not proof of peer identity or isolation.

## Ownership and resource behavior

The trusted producer writes a fresh close-on-exec memfd, then seals writing, growing, shrinking and further seal changes. The receiver consumes its supplied descriptor on every outcome. It checks those permanent seals before sampling file metadata, preventing a size observation from racing a producer's final mutation/sealing. It requires a bounded regular object with exact length, reads by explicit offsets and copies raw pixels into owned memory. It does not map an untrusted buffer, access a pathname or import a GPU buffer.

The receiver accepts at most 15 frames per second using its own monotonic clock. Saturated arrivals are refused before allocating pixel storage and do not advance the sequence. It retains at most two owned pixel buffers; the oldest is dropped before allocating a replacement, and the latest valid buffer survives a replacement allocation/read failure. Maximum retained pixel payload is 7,372,800 bytes. Clearing buffers does not reset the sequence guard.

This is a payload retention bound, not a total memory certificate. Producer buffers, memfd pages, graphics textures, allocator overhead, queued descriptors and process memory require separate accounting and enforcement. The future transport must authenticate its private peer, bound pending descriptor queues and drain/drop stale input without an unbounded work loop. Receiver success does not establish that pixels came from the expected compositor, output, scenario or projection.

## Evidence and remaining integration

Six checks use actual Linux memfd seals and raw pixel reads. They cover immutable data/length, offset-independent reads, mutable/nonregular inputs, 448 single-bit mutations across fixed/correlated/size header fields, zero/replayed sequences, exact record sizes, pixel limits, portrait orientation, two-buffer retention and clearing. A private deterministic clock check tests the exact rate deadline, one nanosecond below it and backwards-time refusal without depending on scheduler speed.

Capture-format conversion, color/alpha fidelity, protocol XML and private Wayland negotiation, authenticated descriptor transport, source-generation lifecycle, input release, rendering visibility, compositor/process identity, resource enforcement and verified teardown are still required by E09/E10. This module makes no full preview, resource or security acceptance claim. See [supervision status](preview-supervision.md) and the original [blueprint](engineering-blueprint.md).
