<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="Torosu" width="128" height="128" />
</p>

<h1 align="center">Torosu</h1>

<p align="center">All the tools you would need for <strong>osu! stable</strong> in the same place.</p>

---

## Features

- **Downloader**: download beatmap sets from a list of links straight into your Songs folder, skipping the ones you already have.
- **Export Collections**: save your collections as beatmap links, or as a list to import on another install.
- **Import Collections**: bring collections from a file into osu!, merged with the ones you already have.
- **Export Song Folder**: filter your library by mode, status, stars, length and more, and export it as beatmap links.
- **Create Packs**: put diffs from any sets, rates and unsubmitted maps too, into a pack of your own.
- **Trainer**: change the rate, the stats, the long notes or the key count of the map selected in osu!. It reads the map straight from the game, so there is nothing to open by hand.

Torosu is made for **osu! stable**. osu!lazer is not supported: the tools read and write osu! stable's files.

## Install

Download the file for your system from the [latest release](https://github.com/Manolo-dl/Torosu/releases/latest). `<v>` is the version, for example `1.0.0`.

### Windows

Run `torosu_<v>_x64-setup.exe`. Windows 10 and 11 (64-bit) are supported.

### Linux

| Distribution | Install |
| --- | --- |
| Ubuntu 22.04+ / Debian 12+ | `sudo apt install ./torosu_<v>_amd64.deb` |
| Fedora | `sudo dnf install ./torosu-<v>-1.x86_64.rpm` |
| Arch Linux and any other | `chmod +x torosu_<v>_amd64.AppImage && ./torosu_<v>_amd64.AppImage` |

osu! is expected to run under Wine, for example with [osu-winello](https://github.com/NelloKudo/osu-winello).

#### Reading osu!'s memory

The Trainer reads the selected map from osu!'s memory. Most distributions only let a program read another one's memory when it has the permission for it. If the Trainer shows *Can't read osu!*, give it to Torosu once:

```sh
sudo setcap cap_sys_ptrace=eip /usr/bin/torosu
```

Run it again after each update: the new version replaces the file, and the permission with it.

An AppImage can't keep that permission. With the AppImage, allow it for every program until the next reboot instead:

```sh
echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope
```

### Updates

Torosu looks for a new version when it starts. When there is one, an icon shows up in the header: open it and choose **Update and restart**. The Windows installer, the AppImage, the `.deb` and the `.rpm` update themselves; otherwise Torosu links to the releases page.

### Coming from osu-tools

Torosu is the new name of osu-tools, and 1.0.0 installs as a new app: uninstall osu-tools, and set the osu! folder and the osu! session cookie again in Torosu.

## First start

Torosu looks for your osu! folder: the one of the running osu!, or the usual install folders. If it can't find it, it asks for it: choose the folder that has `osu!.db`. You can change it later in the settings (the gear in the header), where you can also turn tools off, set up the downloader and the packs, and save the osu! session cookie between sessions.

## Building from source

You need [Rust](https://www.rust-lang.org/tools/install) 1.88 or newer, [Node.js](https://nodejs.org/) 22 or newer, [pnpm](https://pnpm.io/), and the [system dependencies of Tauri](https://tauri.app/start/prerequisites/) for your platform.

```sh
pnpm install
pnpm tauri dev      # run it
pnpm tauri build    # build the installers into src-tauri/target/release/bundle
```

The backend tests run with `cargo test --lib` in `src-tauri`.

## License

Torosu is licensed under the [MIT License](LICENSE). It includes third-party software under its own license, listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

Torosu is not affiliated with osu! or ppy Pty Ltd. "osu!" is a trademark of ppy Pty Ltd.
