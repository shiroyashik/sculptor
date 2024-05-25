## The Sculptor

Unofficial backend V2 for the Minecraft mod [Figura](https://github.com/FiguraMC/Figura).

Implements Ping transmission functionality via Websocket and full avatar upload and download functionality. Currently incomplete and under active development

And also a distinctive feature is the possibility of player identification through the third-party authorization system [Ely.By](https://ely.by/)

### Running with Docker

You will need an already configured Docker with Traefik (you can use any reverse proxy)

1. Create avatars folder (it will store player avatars)
2. Copy Config.example.toml and rename it to Config.toml
3. Copy docker-compose.example.yml and rename to docker-compose.yml
4. Open docker.compose.yml and uncomment the "labels" to work with Traefik and add the container to the network with Traefik.
5. `docker compose up -d` this will build and run the container with 

### Just running

Running this way you won't need WSL when running on Windows, but....
To do this, you will need to reverse proxy port 6665 to your domain with SSL

1. Create avatars folder (it will store player avatars)
2. Copy Config.example.toml and rename it to Config.toml
3. Set up your reverse proxy server
4. `cargo run`

### TODO:
- [ ] Realization of storing profiles in the database
- [ ] Frontend for moderation
- [ ] Autonomous working without reverse proxy server
- [ ] and many other...