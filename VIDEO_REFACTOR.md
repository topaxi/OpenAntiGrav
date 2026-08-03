You are modifying a Rust + wgpu game engine's cutscene/video playback system.

Current implementation:
- Source assets are H.264 videos.
- At build time/runtime they are transcoded into an AV1 cache.
- Playback uses rav1d for AV1 software decoding.
- This adds unnecessary preprocessing, storage overhead, complexity, and removes the ability to use platform-optimized video decoding hardware.

Goal:
Replace the H.264 -> AV1 cache -> rav1d pipeline with a platform-native H.264 playback architecture.

Primary requirements:
- Keep original H.264 assets.
- Remove the mandatory transcoding step.
- Prefer platform-native hardware accelerated decoders when available.
- Maintain a Rust-first architecture.
- Keep the renderer backend as wgpu.
- Do not introduce FFmpeg or GStreamer as mandatory dependencies.
- Provide a software fallback path where platform decoding is unavailable.

Target platforms:

1. Windows
   - Use Media Foundation / native Windows video decoding APIs.
   - Support hardware decode paths exposed through the OS.

2. Linux
   - Prefer VA-API where available.
   - Support AMD and Intel GPUs.
   - Consider Vulkan Video as a future optimization, but do not make it a requirement.

3. SteamOS
   - Treat as Linux.
   - Ensure compatibility with Steam Deck hardware.
   - Prefer Mesa-supported decode paths.

4. macOS
   - Use VideoToolbox.
   - Support Apple Silicon and Intel Macs where practical.

5. Android
   - Use Android MediaCodec.
   - Prefer hardware decode through the Android platform APIs.

6. Optional future:
   - Keep the design flexible enough for Nintendo Switch / Switch 2 homebrew support.
   - Avoid assumptions that make console-specific decoder backends impossible.

Architecture requirements:

Create a decoder abstraction independent of the platform.

Example:

VideoDecoder trait:
- open(source)
- get_video_info()
- submit_packet()
- receive_frame()
- seek()
- flush()

Implement platform backends:

- WindowsDecoder
- LinuxVaapiDecoder
- MacVideoToolboxDecoder
- AndroidMediaCodecDecoder
- SoftwareDecoder fallback

The renderer must not depend on a specific decoder implementation.

Frame handling requirements:

Design the frame pipeline with future zero-copy support in mind, but do NOT make zero-copy a requirement for the first implementation.

Important:
The target content is PSP/PS2-era cutscenes:
- Low resolution (~480p or below)
- Low bitrate
- Limited playback frequency

Therefore:
- A reliable decode -> CPU frame conversion -> wgpu texture upload path is acceptable.
- Prioritize portability, maintainability, and correctness over maximum throughput.
- Do not introduce large amounts of platform-specific complexity just to avoid a small texture upload.

However:
- Keep the internal frame abstraction capable of representing native GPU-backed frames.
- Avoid designing an API that prevents future zero-copy implementations.

Future optimization possibilities:
- Linux:
  - VA-API surfaces
  - DMA-BUF export/import
  - Vulkan external memory paths
- Windows:
  - D3D11/D3D12 video surfaces
- macOS:
  - CVPixelBuffer / Metal texture interop
- Android:
  - AHardwareBuffer / Vulkan interop

These should be optional optimization layers, not prerequisites.

Supported frame formats:
- Prefer supporting common hardware decoder output formats:
  - NV12
  - YUV420
  - RGBA fallback
- Allow shader-based YUV conversion where beneficial.
- Keep color conversion handling separate from decoding logic.

Fallback:

Add a software H.264 decoder backend.

Requirements:
- Prefer a Rust-compatible implementation where practical.
- OpenH264 is acceptable.
- The game must still function without hardware decoding.

Asset pipeline:

Remove:
- AV1 cache generation.
- rav1d playback dependency.

Keep:
- Original H.264 assets.

Prefer:
- Standard MP4/H.264 assets:
  - H.264 Baseline/Main/High profile
  - 8-bit
  - 4:2:0 chroma

Tasks:

1. Analyze the existing video pipeline.
2. Design the decoder abstraction.
3. Remove the AV1 conversion/cache pipeline.
4. Implement platform backends incrementally.
5. Keep wgpu rendering integration clean.
6. Add tests:
   - decoder initialization
   - frame extraction
   - playback timing
   - seeking
   - fallback behavior
7. Document:
   - supported platforms
   - required dependencies
   - hardware acceleration support
   - limitations

Implementation priorities:

1. Correctness and portability.
2. Clean Rust abstraction.
3. Minimal dependencies.
4. Platform acceleration.
5. Zero-copy optimizations only where they provide measurable benefit.

Do not replace one complicated codec pipeline with another.
The objective is a game-engine-quality video system that uses the platform's optimized media capabilities while remaining maintainable and portable.
