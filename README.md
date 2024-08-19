# The Sculptor

[![Rust](https://github.com/shiroyashik/sculptor/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/shiroyashik/sculptor/actions/workflows/rust.yml)

Unofficial backend V2 for the Minecraft mod [Figura](https://github.com/FiguraMC/Figura).

Is a worthy replacement for the official version. Realized all the functionality that can be used during the game.

And also a distinctive feature is the possibility of player identification through third-party authentication providers (such as [Ely.By](https://ely.by/))

## Launch

To run it you will need a configured reverse proxy server.

Make sure that the reverse proxy you are using supports WebSocket and valid certificates are used for HTTPS connections.

> [!IMPORTANT]
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
# Clone the latest release
git clone https://github.com/shiroyashik/sculptor.git
# or a dev release
git clone --branch dev https://github.com/shiroyashik/sculptor.git
# Enter the folder
cd sculptor
# Copy Sculptor configuration file
cp Config.example.toml Config.toml
# Edit configuration file for your needs
nano Config.toml
# Build it in release mode for better performance
cargo build --release
```

## Public server

[![Server status](https://up.shsr.ru/api/badge/1/status?upLabel=Online&downLabel=Offline&label=Server+status)](https://up.shsr.ru/status/pub)

I'm keeping the public server running at the moment!

You can use it if running your own Sculptor instance is difficult for you.

> figura.shsr.ru

For reasons beyond my control, the server is not available in some countries.

## Contributing

If you have ideas for new features, have found a bug, or want to suggest improvements,
please create an [issue](https://github.com/shiroyashik/sculptor/issues)
or contact me directly via Discord (@shiroyashik).

If you are a Rust developer, you can modify the code yourself and request a Pull Request:

1. Fork the repository.
2. Create a new branch for your features or fixes.
3. Submit a PR.

Glad for any help from ideas to PRs.

#### P.S.

The [“master”](https://github.com/shiroyashik/sculptor/tree/master) branch contains the source code of the latest release. A [“dev”](https://github.com/shiroyashik/sculptor/tree/dev) branch is used for development.

## License

The Sculptor is licensed under [GPL-3.0](LICENSE)
