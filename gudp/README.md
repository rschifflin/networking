# GUDP

Simple network protocol implementation based on Glenn Fiedler's [game networking articles](https://gafferongames.com/post/virtual_connection_over_udp/).
Provides a virtual connection interface for sending UDP packets over with congestion control.

Does no fragmentation/reassembly- it's still a UDP packet protocol.
On top of this _datagram_ protocol we can build a _message protocol_

## Architecture

Ideally, a simple wrapper daemon over GUDP would provide a single entrypoint via IPC
for any application wishing to open a GUDP connection. In reality, any individual program
will likely include it as a library to run on its own thread.

Components:
- GUDP Daemon:
    The event loop that manages all GUDP connections. Run in its own thread.
    Handles the state machine for each individual connection and evented io.
- GUDP Listener:
    App-facing listener object with accept interface.
    Corresponds to a 'passive open' with many peers multiplexed onto a single UDP socket.
- GUDP Connection:
    App-facing connection object with send/recv interface.
    Wraps a UDP socket and provides a virtual connection to a peer.

## Reading and locking - Naive approach
Each connection includes a pair of read/write buffers shared between the daemon thread
and the application thread. The daemon thread pushes socket reads into the read buffer while
the app thread pulls reads out, and so the two threads may contend for the same buffer.
Additionally, we would like app connections to be cloneable, meaning potentially n consumers will pull from the same read buffer.
(Note: the daemon thread will be the sole source of pushing to the read buffer)

In order to handle resource contention, consider the following naive policy:

Each consumer will first acquire a lock on the read buffer.
Whoever acquires the lock will then check the following read condition:
- The buffer has reads available (count > 0),

If the condition _fails_, the consumer will then wait on the read buffer condvar.
If the condition succeeds, the consumer will attempt to read from the buffer.

If after the consumer works on the buffer, the read condition still holds,
it will signal the condvar to wake up the next consumer before giving up its lock.

### Caveat - The status flags
There is a slight wrinkle to the above approach. At any point we want to be able to end the connection via setting the `status` atomic.

Whenever the producer on the daemon thread sees a final status, it will be responsible for notifying _all_ consumers once again (AFTER acquiring the read buffer lock!!), even if the read condition does not hold. Thus, consumers attempting to read must actually check two conditions:
- There are reads available (count > 0) *OR*
- The connection is finished according to the status atomic.

Note: Although the producer will acquire the read buffer lock before signalling to the consumers a final status (aka no more reads will be coming), in general consumers are _NOT_ restricted by the read buffer mutex when setting the status flags themselves. This is allowed due to an invariant which *must hold*:

### The status invariant

The status flags must NEVER be changed from a final state back to a working state. Any number of consumers may set a final state for any reason freely without acquiring any locks. However, nobody may ever set a state back to working from final! This is why an atomic is needed- to prevent unwittingly overwriting a working->final state change with a working -> working state change.

This invariant is enforced by the state::Status api, which makes sure any
writes do not trample the bits which represent a closed state.

## The virtual connection
There are a lot of subtle edge cases when handling virtual connections and properly freeing resources.
The following is a general description:
  On the daemon thread, connections are tracked in hashmap of Mio Token -> socket object,
  where socket objects contain the underlying Mio UdpSocket and various connection properties such as peer type.

  Connections created via a direct connect() call are an ACTIVE OPEN and provide a connection directly to the app thread, without requiring a listener.
  On the daemon thread, they are marked by a socket of PeerType::Active.
  This variant includes the state machine of the single directly-connected peer.
  Whenever the underlying Mio UdpSocket closes (either the app hangs up,
  the peer hangs up, or a fatal io error occurs), the following actions occur:
    - The status flag for the connection is set to closed (see Reading and Locking above)
    - The mio udpsocket is unregistered from the evented io poller
    - The token|socket hashmap entry is dropped
  This gracefully cleans up the connection resources, and allows the app thread to still drain its read queue until it observes the now-closed status,
  at which point it knows no future reads/writes are possible and can be safely drop its half.

  Connections created via calls to listen() are a PASSIVE OPEN and provide a Listener to the app thread.
  On the daemon thread, they are marked by a socket of PeerType::Passive.
  This variant includes a hash of currently connected peer addresses to their state machines,
  a marker set of peer addresses with pending writes,
  and a listen option whose presence indicates listening for new peers and whose value is the machinery needed to build a new connection.

  When the app thread drops its Listener, the following actions occur on the daemon thread:
    - The listen option is set to None.
    - IF there are no peers left still connected:
      - the mio udpsocket is unregistered from the evented io poller
      - The token|socket hashmap entry is dropped

  For each peer, whenever its virtual connection is terminated gracefully (app hup or peer hup), the following actions occur:
    - The status flag for the connection is set to closed (see Reading and Locking above)
    - The peer is removed from the map of peer state machines
    - IF there are no peers left still connected:
      - the mio udpsocket is unregistered from the evented io poller
      - The token|socket hashmap entry is dropped

  For any peer, whenever its virtual connection has a fatal io error, the following actions occur:
    - The status flag for ALL peers is set to closed (see Reading and Locking above)
    - the mio udpsocket is unregistered from the evented io poller
    - The token|socket hashmap entry is dropped (including ALL peers)

  This gracefully cleans up the connection resources, allows the app thread connections to still drain their read queues, and
  removes listener io resources only if and exactly when they have no connections remaining.

  NOTE: When the app thread drops a connection, it sets the app hup status. While this does not trigger an event directly on the daemon event loop,
  eventually the write inactivity will cause a heartbeat, and when sending a heartbeat the daemon checks for closed connections and will clean up
  as specified above.

## Timers
  The virtual connection is temporal- a connection to a peer is implicitly assumed whenever datagrams are being received from said peer.
  After a period of inactivty, the peer is disconnected. To keep the connection alive, a regular heartbeat interval timer sends out
  empty updates (if no other sends have occured since the last heartbeat). Since the goal is a best-effort reliability protocol,
  we should ensure at least n heartbeats are sent within the timeout window, where n-1 is the limit of packet loss we're willing to tolerate.
