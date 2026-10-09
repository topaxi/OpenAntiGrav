// Movies on the page: the browser's WebCodecs VideoDecoder, driven by the
// module (oag_game's movie/webcodecs.rs) one H.264 access unit at a time. A
// decoded frame is copied out as planes and handed back when the module asks;
// nothing here waits, the decode runs on the browser's own threads. See
// docs/tools/web.md, "Movies".

const movies = new Map();
let nextId = 1;

function configure(movie) {
  const generation = ++movie.generation;
  movie.slots = [];
  movie.flushing = false;
  movie.decoder = new VideoDecoder({
    output: (frame) => {
      // A frame from before a reset belongs to playback that no longer exists.
      if (generation !== movie.generation) { frame.close(); return; }
      try {
        const rect = frame.visibleRect;
        // A frame with no format (one left on the GPU) copies out only as RGB,
        // which the module converts back.
        const options = frame.format ? {} : { format: "RGBA" };
        const slot = {
          format: frame.format ?? "RGBA", width: rect.width, height: rect.height, bytes: null, layout: null,
          // The matrix an RGB frame was converted with, for the module to invert.
          matrix: frame.colorSpace?.matrix ?? null,
        };
        const bytes = new Uint8Array(frame.allocationSize(options));
        // Queued now, filled when the copy lands: the copies are asynchronous,
        // and the module takes frames in the order the decoder put them out.
        movie.slots.push(slot);
        frame.copyTo(bytes, options).then(
          (layout) => { slot.layout = layout; slot.bytes = bytes; },
          (why) => { if (generation === movie.generation) movie.error = `copyTo: ${why}`; },
        ).finally(() => frame.close());
      } catch (why) {
        movie.error = `copying a frame out: ${why}`;
        frame.close();
      }
    },
    error: (why) => { if (generation === movie.generation) movie.error = String(why); },
  });
  movie.decoder.configure({
    codec: movie.codec, codedWidth: movie.width, codedHeight: movie.height,
    optimizeForLatency: true,
  });
}

// A handle, or why there is none.
window.oagMovieOpen = (codec, width, height) => {
  if (typeof VideoDecoder === "undefined") return "this browser has no WebCodecs VideoDecoder";
  const movie = { codec, width, height, generation: 0, error: null };
  try { configure(movie); } catch (why) { return `configuring ${codec}: ${why}`; }
  const id = nextId++;
  movies.set(id, movie);
  return id;
};

window.oagMovieDecode = (id, bytes, index, key) => {
  const movie = movies.get(id);
  // 30 Hz in microseconds; the module counts frames, this is only required.
  movie.decoder.decode(new EncodedVideoChunk({
    type: key ? "key" : "delta", timestamp: Math.round(index * 1e6 / 30), data: bytes,
  }));
};

window.oagMovieFlush = (id) => {
  const movie = movies.get(id);
  const generation = movie.generation;
  movie.flushing = true;
  movie.decoder.flush().then(
    () => { if (generation === movie.generation) movie.flushing = false; },
    (why) => { if (generation === movie.generation) { movie.flushing = false; movie.error = `flush: ${why}`; } },
  );
};

// Pictures inside the decoder, on their way out of it, or waiting to be taken.
window.oagMovieBacklog = (id) => {
  const movie = movies.get(id);
  return movie.decoder.decodeQueueSize + movie.slots.length + (movie.flushing ? 1 : 0);
};

window.oagMovieTake = (id) => {
  const movie = movies.get(id);
  return movie.slots.length && movie.slots[0].bytes ? movie.slots.shift() : null;
};

window.oagMovieError = (id) => movies.get(id)?.error ?? null;

// Back to frame zero: a fresh decoder, as a loop's wrap or a reopened menu asks.
window.oagMovieReset = (id) => {
  const movie = movies.get(id);
  if (movie.decoder.state !== "closed") movie.decoder.close();
  configure(movie);
};

window.oagMovieClose = (id) => {
  const movie = movies.get(id);
  movies.delete(id);
  movie.generation++;
  if (movie.decoder.state !== "closed") movie.decoder.close();
};
