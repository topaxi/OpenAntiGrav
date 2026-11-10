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
        try {
          return new Uint8Array(new FileReaderSync().readAsArrayBuffer(file.slice(offset, offset + length)));
        } catch (error) {
          self.oagFatal("disc-unreadable", `read of ${length} bytes at ${offset}: ${error}`);
          throw error;
        }
      },
    };
  }
  // A panic here has no page to draw on: hand it to the page's `oagFatal`
  // (fault.js), which also ends every other thread.
  self.oagFatal = (kind, detail) => postMessage({ fatal: { kind, detail } });
  let exports, glueModule;
  try {
    glueModule = await import(glue);
    exports = await glueModule.default({ module_or_path: module, memory });
    glueModule.oag_worker_entry(id);
  } catch (error) {
    // A trap or a failed start-up on a worker: an async handler's rejection
    // reaches nobody otherwise.
    self.oagFatal("trap", `${error?.name}: ${error?.message}\n${error?.stack ?? ""}`);
    close();
    return;
  }
  // Hands this thread's stack and TLS block back to the shared memory, which
  // never shrinks: without it every race load would leak them.
  exports.__wbindgen_thread_destroy();
  close();
};
