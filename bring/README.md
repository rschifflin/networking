# bRing

A ring buffer for binary blobs of data.

The bounded buffer does not reallocate and any attempts to push return either Some of the size of the source buffer pushed or None in case of overlow.

The unbounded buffer grows using the underlying vec's strategy if needed on push. Pushing always returns the size of the source buffer pushed.
