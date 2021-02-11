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

## Reading and locking - Naive approach
Each connection is a pair of read/write buffers shared between the daemon thread
and the user (service) thread. The daemon thread pushes socket reads into the read
buffer while the user thread pulls reads out, and so the two threads may contend for the same buffer. Additionally, we would like user sockets to be easily sharable, meaning potentially n consumers will pull from the same read buffer (the daemon thread will be the sole source of pushing to the read buffer).

In order to handle resource contention, the following policy is observed:

Each consumer will first acquire a lock on the read buffer.
Whoever acquires the lock will then check the following read condition:
- The buffer has reads available (count > 0),

If the condition _fails_, the consumer will then wait on the read buffer condvar.
If the condition succeeds, the consumer will attempt to read from the buffer.

If after the consumer works on the buffer, if the read condition still holds,
it will signal the condvar to wake up the next consumer before giving up its lock.

### Caveat - The status flags
There is a slight wrinkle to the above approach. At any point we want to be able to end the connection via setting the `status` atomic.

Whenever the producer on the daemon thread sees a final status, it will be responsible for notifying _all_ consumers once again (AFTER acquiring the read buffer lock!!), even if the read condition does not hold. Thus, consumers attempting to read must check:
- There are reads available (count > 0) *OR*
- The connection is finished.

Note that althought the producer will acquire the read buffer lock before singalling to the consumers a final status (aka no more reads will be coming), in general consumers are _NOT_ restricted by the read buffer mutex when setting the status flags themselves. This is allowed due to an invariant which *must hold*:

### The status invariant

The status flags must NEVER be changed from a final state back to a working state. Any number of producers/consumers may set a final state for any reason freely without acquiring any locks. However, nobody may ever set a state back to working from final! This is why an atomic is needed- to prevent unwittingly overwriting a working->final state change with a working -> working state change.

This invariant is enforced largely by segregation of bits in the status flags.
Low-order bits may indicate working statuses such as the current congestion level.
High-order bits always indicate final statuses such as various failure modes.
Any status writes to the low order bits must atomically compare < the minimum error bits before swapping in.
Any status writes to the high order bits may always `fetch_or` the high-order error bit they wish to set.

For type safety, the `status` atomic sits under a wrapper type that enforces these invariants on any attempt to read/write.
