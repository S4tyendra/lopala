# Lopala

A zero-config, real-time Web Operating System that drops onto your local machine and securely exposes a full desktop/terminal experience over the web.

Technical details on why it exists: [satyendra.in/blog/why-i-built-lopala](https://satyendra.in/blog/why-i-built-lopala)

<video src="assets/recording.webm" controls autoplay loop muted style="max-width: 100%; border-radius: 8px;"></video>

## Drop It In (Installation)

The easiest way to get it running:

```bash
curl -fsSL https://raw.githubusercontent.com/s4tyendra/lopala/main/install.sh | bash
````

**Manual / Binary Release:**
Alternatively, grab the static binary for your architecture directly from the **Releases** page:

```bash
chmod +x lopala
sudo mv lopala /usr/bin/
lopala
```

## Usage & Flags

Just run `lopala` to start the server. Need to bypass a firewall? Use the tunnel.

  - `--tunnel`: Instantly exposes Lopala to the public internet via Cloudflare. Auto-downloads `cloudflared` securely if it's not present on your system.
  - `--port <PORT>`: The local port to bind and listen on. Defaults to `8080`. (If using `--tunnel` without a port, it picks a random ephemeral port between `40000` and `60000`).
  - `--max-users`: Maximum concurrent users allowed in the session at once. Defaults to `10`.

## The Features

  - **PTY Terminal:** Full PTY support with multi-user multiplexed sync that *just works* out of the box. Includes professional toolbars.
  - **Multiplayer Mode:** Fully collaborative, interactive sessions for terminal and UI.
  - **File Manager:** Upload(better upload than just POST), download, rename, delete, create, and edit. Direct file downloads straight from the terminal.
  - **Text Editor:** Built-in workspace editor with synntax highlighting.
  - **Task Manager:** Live monitoring and process management.
  - **Screen Stream:** Live screenshot/streaming of the attached display (if you have one).
  - **Auth:** Secure PIN-based authentication.

## Why for DevOps?

Lopala gives you a zero-configuration web GUI, a fully integrated PTY terminal, live task manager, and file editor over *any* headless server. It's incredibly useful for homelabs, remote infrastructure troubleshooting, acting as a drop-in emergency dashboard over a tunnel when native SSH acts up, or just letting your senior dev securely jump in to fix your broken environment.

## Contributions & Issues

  - **Windows Support:** Need help with Windows testing. If you can test or fix issues on Windows, PRs are highly welcome.
  - **Bugs / Feedback:** Open an Issue.
  - **Changes / Fixes:** Submit a Pull Request.

## Security

As of building this, there are no known CVEs for the libraries and dependencies used.

## License

MIT — **Do whatever you like.** I don't care, and I can't guarantee anything.

See the [LICENSE](https://www.google.com/search?q=LICENSE) file for more information. (Provided "AS IS" without warranty of any kind).

## 🤍 Support the Project

Lopala is MIT licensed, but if you use the code, concept, or design engineering tokens in your own project, a **Star on GitHub** is the best way to say thanks.

[⭐ Star github.com/s4tyendra/lopala](https://github.com/s4tyendra/lopala)
