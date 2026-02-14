# Lopala

A highly-performant, real-time Web Operating System that runs on your local machine and exposes it securely over a web interface. 

Technical details on why it exists: [satyendra.in/blog/why-i-built-lopala](https://satyendra.in/blog/why-i-built-lopala)

<video src="assets/recording.webm" controls autoplay loop muted style="max-width: 100%; border-radius: 8px;"></video>

## Usage & Installation

Download the binary for your architecture from the **Releases** page. Keep it simple.

```bash
chmod +x lopala
sudo mv lopala /usr/bin/
lopala
```

## Flags

- `--max-users`: How many maximum concurrent users can connect at a time? (defaults to 10)
- `--port <PORT>`: The local port to bind and listen on. Defaults to `8080`.
- `--tunnel`: Instantly exposes Lopala to the public internet via Cloudflare. If no port is passed, it picks a random ephemeral port between `40000` and `60000`. It will auto-download `cloudflared` securely if it's not present on your system.

## Why for DevOps?

Lopala gives you a zero-configuration web GUI, full integrated PTY terminal, live task manager, and file editor over any headless server. Extremely useful for homelabs, remote infrastructure troubleshooting, or acting as a drop-in emergency dashboard over a tunnel when native SSH access acts up.

## Contributions & Issues

- **Windows Support:** Need help with Windows testing. If you can test or fix issues on Windows, contributions are highly welcome.
- **New Features/Feedback/Bugs:** Open an Issue.
- **Changes/Fixes:** Submit a Pull Request.

## Security

As of building this, there are no known CVEs for the libraries and dependencies used.

## License

MIT — **Do whatever you like.** I don't care, and I can't guarantee anything. 

See the [LICENSE](LICENSE) file for more information. (Provided "AS IS" without warranty of any kind).

### 🤍 Support the Project
Lopala is MIT licensed, but if you use the code, concept, or design engineering tokens in your own project, a **Star on GitHub** is the best way to say thanks. 
[⭐ Star github.com/s4tyendra/lopala](https://github.com/s4tyendra/lopala)
