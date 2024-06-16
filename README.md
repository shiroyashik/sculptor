# The Sculptor

[![Rust](https://github.com/shiroyashik/sculptor/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/shiroyashik/sculptor/actions/workflows/rust.yml)

Unofficial backend V2 for the Minecraft mod [Figura](https://github.com/FiguraMC/Figura).

Implements Ping transmission functionality via Websocket and full avatar upload and download functionality. Currently incomplete and under active development

And also a distinctive feature is the possibility of player identification through the third-party authorization system [Ely.By](https://ely.by/)

## Usage

### Docker

You will need an already configured Docker with Traefik (you can use any reverse proxy)

1. Create avatars folder (it will store player avatars)
2. Copy Config.example.toml and rename it to Config.toml
3. Copy docker-compose.example.yml and rename to docker-compose.yml
4. Open docker.compose.yml and uncomment the "labels" to work with Traefik and add the container to the network with Traefik.
5. `docker compose up -d` this will build and run the container with 

### Native

Running this way you won't need WSL when running on Windows, but....

To do this, you will need to reverse proxy port 6665 to your domain with SSL

1. Create avatars folder (it will store player avatars)
2. Copy Config.example.toml and rename it to Config.toml
3. Set up your reverse proxy server
4. `cargo run`

> [!IMPORTANT]
> NGINX requires additional configuration to work with websocket!

## Public server

[![Server status](https://up.shsr.ru/api/badge/1/status?upLabel=Online&downLabel=Offline&label=Server+status)](https://up.shsr.ru/status/pub)

I'm keeping the public server running at the moment!

You can use it if running your own Sculptor instance is difficult for you.

> figura.shsr.ru

For reasons beyond my control, the server is not available in some countries.
