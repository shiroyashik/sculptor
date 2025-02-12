 - English
 - [Русский](README.ru.md)

# The Sculptor

[![Push Dev](https://github.com/shiroyashik/sculptor/actions/workflows/dev-release.yml/badge.svg?branch=dev)](https://github.com/shiroyashik/sculptor/actions/workflows/dev-release.yml)
![maintenance-status](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

Unofficial backend for the Minecraft mod [Figura](https://github.com/FiguraMC/Figura).

Is a worthy replacement for the official version. Realized all the functionality that can be used during the game.

And also a distinctive feature is the possibility of player identification through third-party authentication providers (such as [Ely.By](https://ely.by/))

## Public server

[![Server status](https://up.shsr.ru/api/badge/1/status?upLabel=Online&downLabel=Offline&label=Server+status)](https://up.shsr.ru/status/pub)

I'm keeping the public server running at the moment!

You can use it if running your own Sculptor instance is difficult for you.

To connect, simply change **Figura Cloud IP** in Figura settings to the address below:

> figura.shsr.ru

Authentication is enabled on the server via: Mojang(Microsoft) and [Ely.By](https://ely.by/)

For reasons beyond my control, the server is not available in some countries.


## Launch

To run it you will need a configured reverse proxy server.

Make sure that the reverse proxy you are using supports WebSocket and valid certificates are used for HTTPS connections.

> [!WARNING]
> NGINX requires additional configuration to work with websocket!

### Docker

For the template you can use [docker-compose.example.yml](docker-compose.example.yml)

It assumes you will be using Traefik as a reverse proxy, if so uncomment the lines and add Sculptor to the network with Traefik.

Copy [Config.example.toml](Config.example.toml) change the settings as desired and rename to Config.toml

That's enough to start Sculptor.

### Pre-Built

See the [pre-built archives](https://github.com/shiroyashik/sculptor/releases/latest)

### Build from source

A pre-installed Rust will be required for the build

```sh
# Clone the pre-release
git clone https://github.com/shiroyashik/sculptor.git
# or clone specific version
git clone --depth 1 --branch v0.4.0 https://github.com/shiroyashik/sculptor.git
# Enter the folder
cd sculptor
# Copy Sculptor configuration file
cp Config.example.toml Config.toml
# Edit configuration file for your needs
nano Config.toml
# Build it in release mode for better performance
cargo build --release
# or run from cargo
cargo run --release
```

#### Compiling from the `master` Branch

> [!IMPORTANT]
> Installing Sculptor directly from the `master` branch is **not recommended** for most users. This branch contains pre-release code that is actively being developed and may include broken or unstable features. Additionally, using the `master` branch could potentially cause issues with data migration when upgrading to future stable releases.
>
> If you still choose to use the `master` branch, please be aware that you may encounter bugs or unexpected behavior. Your feedback and bug reports are highly appreciated. However, for a more stable and reliable experience, we strongly advise using the **latest official release** instead.

## Contributing
![Ask me anything!](https://img.shields.io/badge/Ask%20me-anything-1abc9c.svg)
on
[![Telegram](https://badgen.net/static/icon/telegram?icon=telegram&color=cyan&label)](https://t.me/shiroyashik)
or
![Discord](https://badgen.net/badge/icon/discord?icon=discord&label)

If you have ideas for new features, have found a bug, or want to suggest improvements,
please create an [issue](https://github.com/shiroyashik/sculptor/issues)
or contact me directly via Discord/Telegram (**@shiroyashik**).

If you are a Rust developer, you can modify the code yourself and request a Pull Request:

1. Fork the repository.
2. Create a new branch for your features or fixes.
3. Submit a PR.

Glad for any help from ideas to PRs. ❤

## License

The Sculptor is licensed under [GPL-3.0](LICENSE)
