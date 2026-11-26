# oag-thread

A closure on another thread on every target: `std::thread` natively, a Web
Worker on the module's shared memory in the browser, where the page's thread must
never block. Natively, `poller` asks a number once a period off the frame loop.

No workspace dependency; the race load, the music fetch and the audio render loop
sit above it. See [web](../../docs/tools/web.md).
