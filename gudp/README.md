# GUDP

Simple network protocol implementation based on Glenn Fiedler's [game networking articles](https://gafferongames.com/post/virtual_connection_over_udp/).
Provides a virtual connection interface for sending UDP packets over with congestion control.

Does no fragmentation/reassembly- it's still a UDP packet protocol.
On top of this _packet_ protocol we will next build a _message protocol_
