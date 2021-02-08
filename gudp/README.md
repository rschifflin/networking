# GUDP

Simple network protocol implementation based on Glenn Fiedler's [game networking articles](https://gafferongames.com/post/virtual_connection_over_udp/).
Provides a virtual connection interface for sending UDP packets over with congestion control.

Does no fragmentation/reassembly- it's still a UDP packet protocol.
On top of this _packet_ protocol we will next build a _message protocol_

## Architecture

Ideally, a simple wrapper daemon over GUDP would provide a single entrypoint via IPC
for any application wishing to open a GUDP connection. In reality, any individual program
will likely include it as a library to run on its own thread.

Components:
  GUDP Daemon:
    The event loop that manages all GUDP connections. Run in its own thread.
    Handles the perf counters for each individual connection.
  GUDP Connection:
    User-facing connection object with send/recv interface.
    Wraps a UDP socket. The UDP socket is placed on daemon event loop

