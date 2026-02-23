# Installation

There are several ways to install morph. Pick the one that works best for you.

## Quick Install (Recommended)

Pre-built binaries are available for all major platforms via [cargo-dist](https://opensource.axo.dev/cargo-dist/). These one-liners download the latest release and install it for you.

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/alvinreal/morph/releases/latest/download/morph-installer.sh | sh
```

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/alvinreal/morph/releases/latest/download/morph-installer.ps1 | iex"
```

## Package Managers

### Homebrew (macOS / Linux)

```bash
brew install alvinreal/tap/morph
```

### cargo binstall

If you have [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) installed, it will download pre-built binaries instead of compiling from source:

```bash
cargo binstall morph
```

### cargo install

Build and install directly from [crates.io](https://crates.io/crates/morph-cli):

```bash
cargo install morph-cli
```

> **Note:** This compiles from source, which requires a working Rust toolchain (1.70+) and may take a minute or two.

## Manual Download

Pre-built binaries for every release are available on the [GitHub Releases](https://github.com/alvinreal/morph/releases) page.

1. Download the archive for your platform:

   | Platform              | Archive                                     |
   |-----------------------|---------------------------------------------|
   | macOS (Apple Silicon) | `morph-aarch64-apple-darwin.tar.xz`         |
   | macOS (Intel)         | `morph-x86_64-apple-darwin.tar.xz`          |
   | Linux (x86_64, glibc) | `morph-x86_64-unknown-linux-gnu.tar.xz`    |
   | Linux (x86_64, musl) | `morph-x86_64-unknown-linux-musl.tar.xz`    |
   | Linux (ARM64)         | `morph-aarch64-unknown-linux-gnu.tar.xz`    |
   | Windows (x86_64)      | `morph-x86_64-pc-windows-msvc.zip`          |

2. Extract the archive:

   ```bash
   # macOS / Linux
   tar -xf morph-<target>.tar.xz

   # Windows
   # Use Explorer or: Expand-Archive morph-x86_64-pc-windows-msvc.zip
   ```

3. Move the binary to a directory in your `PATH`:

   ```bash
   # macOS / Linux
   mv morph /usr/local/bin/

   # Or to a user-local directory
   mv morph ~/.local/bin/
   ```

4. Verify the installation:

   ```bash
   morph --version
   ```

## Build from Source

```bash
git clone https://github.com/alvinreal/morph.git
cd morph
cargo build --release
```

The binary will be at `target/release/morph`. Copy it to a directory in your `PATH`:

```bash
cp target/release/morph /usr/local/bin/
```

### Requirements

- Rust 1.70 or later (install via [rustup](https://rustup.rs/))
- A C linker (usually already present on macOS/Linux; on Windows, install the MSVC build tools)

## Shell Completions

morph can generate completions for your shell. Run the appropriate command and follow the instructions for your shell below.

### Bash

```bash
morph --completions bash > ~/.local/share/bash-completion/completions/morph

# Or system-wide:
sudo morph --completions bash > /etc/bash_completion.d/morph
```

Restart your shell or run `source ~/.bashrc` to activate.

### Zsh

```bash
morph --completions zsh > ~/.zfunc/_morph
```

Make sure `~/.zfunc` is in your `fpath`. Add this to your `~/.zshrc` (before `compinit`):

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

Restart your shell or run `source ~/.zshrc` to activate.

### Fish

```bash
morph --completions fish > ~/.config/fish/completions/morph.fish
```

Completions are loaded automatically on the next shell session.

### PowerShell

```powershell
morph --completions powershell >> $PROFILE
```

Restart PowerShell to activate.

### Elvish

```bash
morph --completions elvish >> ~/.config/elvish/rc.elv
```

Restart Elvish to activate.

## Updating

### Quick installers

Re-run the same install command — it will overwrite the existing binary with the latest version.

### Homebrew

```bash
brew upgrade morph
```

### cargo binstall / cargo install

```bash
cargo binstall morph        # downloads new pre-built binary
# or
cargo install morph-cli     # rebuilds from source
```

### Manual

Download the latest release from the [Releases page](https://github.com/alvinreal/morph/releases) and replace the existing binary.

## Troubleshooting

### "command not found" after install

Make sure the installation directory is in your `PATH`. Common locations:

- `/usr/local/bin` (most systems)
- `~/.local/bin` (user-local installs)
- `~/.cargo/bin` (cargo install)

Check with:

```bash
echo $PATH
which morph
```

If `~/.cargo/bin` is missing from your PATH, add it to your shell profile:

```bash
# bash / zsh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc

# fish
fish_add_path ~/.cargo/bin
```

### Permission denied (macOS / Linux)

Make sure the binary is executable:

```bash
chmod +x /usr/local/bin/morph
```

### macOS Gatekeeper warning

If macOS shows "cannot be opened because the developer cannot be verified":

1. Open **System Settings → Privacy & Security**
2. Scroll down — you'll see a message about morph being blocked
3. Click **"Allow Anyway"**
4. Run `morph` again and confirm

Alternatively, remove the quarantine attribute:

```bash
xattr -d com.apple.quarantine /usr/local/bin/morph
```

### Windows SmartScreen warning

If Windows Defender SmartScreen blocks the executable:

1. Click **"More info"**
2. Click **"Run anyway"**

This happens because the binary is not code-signed. It is safe to allow.

### Build failures (cargo install)

If `cargo install morph-cli` fails:

- Make sure your Rust toolchain is up to date: `rustup update`
- On Linux, you may need development headers: `sudo apt install pkg-config libssl-dev` (Ubuntu/Debian)
- Try using `cargo binstall morph` to download a pre-built binary instead
