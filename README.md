# Lopala OS 🚀

A highly-performant, real-time distributed Web Operating System wrapped in a native feel. Lopala seamlessly bridges your underlying Operating System to the Web with high-fidelity interfaces, zero-FCP boot sequences, and collaborative multi-user applications—all delivered through purely native tooling.

## Key Features

- **Fluid Mac-Style Boot & Login:** Instant initial payload load times (zero-FCP) featuring a beautifully orchestrated CSS boot sequence. 
- **Tabbed Collaborative Code Editor:** A deeply integrated CRDT-based editor workspace powered by CodeMirror 6. Changes, active documents, and even user carets natively sync across all connected endpoints in real-time.
- **Activity Monitor (Task Manager):** A clean zero-spin process tracker modeled after professional system activity monitors.
- **Spotlight Search & "Liquid Glass" Rendering:** Access to ultra-fast local indexing leveraging the robust ripgrep (`rg`) engine behind a smooth WebGL glassmorphism overlay.
- **Native Tunneling & Exposure Check:** Seamless command-line flags to broadcast your environment instantly via an automated `cloudflared` pipeline.
- **File Manager & Uploader:** Rich interaction interface mapping the active filesystem, handling everything from contextual editor launches to massive chunked large file uploads.

## Usage

Start the backend environment and automatically host the bundled OS front-end using Cargo:

```bash
# Starts the OS locally on port 8080
cargo run -- --port 8080

# Starts the OS and tunnels it to a public URL directly
cargo run -- --tunnel
```

If the port is omitted and `--tunnel` is provided, Lopala will automatically select a random port between `40000` and `60000` to prevent exposure conflicts. Lopala takes care of caching and downloading architectural builds of `cloudflared` so tunneling remains a frictionless experience on any Linux and ARM system.

### Build and Package

If you need a distributed binary, execute the `build.sh` pipeline which concurrently prepares native x86/ARM executables via Cross tools integrations:

```bash
chmod +x build.sh
./build.sh
```

## Screen Recording
Watch Lopala OS in action inside the [assets/](./assets/) directory.
