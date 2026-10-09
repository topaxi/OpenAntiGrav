// One thread of the game: the module's own code on the page's shared memory,
// running the closure `id` names (`oag_thread::web`). Started by
// `oagSpawnWorker` in main.js. See docs/tools/web.md, "Threads".
self.onmessage = async ({ data: { glue, module, memory, id, file } }) => {
  // The disc reader for this thread: FileReaderSync, which a worker may use
  // and which hands back the bytes as they are (web_image.rs reads it).
  if (file) {
    self.oagImageReader = {
      size: file.size,
      read(offset, length) {
        return new Uint8Array(new FileReaderSync().readAsArrayBuffer(file.slice(offset, offset + length)));
      },
    };
  }
  // A panic here has no page to draw on: hand it to the page's `oagFatal`.
  self.oagFatal = (cause) => postMessage({ fatal: cause });
  const glueModule = await import(glue);
  const exports = await glueModule.default({ module_or_path: module, memory });
  glueModule.oag_worker_entry(id);
  // Hands this thread's stack and TLS block back to the shared memory, which
  // never shrinks: without it every race load would leak them.
  exports.__wbindgen_thread_destroy();
  close();
};
