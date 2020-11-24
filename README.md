# ![Personal forum](docs/img/banner_full.png)

**Personal forum** is a small client-server web app for communication. It's
something between a forum and a chat room, allowing you to create multiple
rooms, and start multiple threads in each room. It tries to emphasize user
privacy and to reduce the amount of traffic between the server and its clients.

Check out the [demo page][demo-page].

This project is hosted on [GitHub](https://github.com/pandrei7/personal_forum).

![An open room with a conversation](docs/img/example_room.png)

## Running and developing

This web app is split in two parts: a backend and a frontend.
The server is written in Rust and it interacts with a PostgreSQL database.
The frontend is written in plain JavaScript, HTML, and CSS.
[Tera](https://tera.netlify.app/docs) templates are also used.

Personal forum's backend server is a Rust program built with the
[Rocket](https://rocket.rs) framework. To run the server locally, you need a
nightly version of the Rust compiler and
[Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).
You also need to
[create a PostgreSQL database](https://www.freecodecamp.org/news/how-to-get-started-with-postgresql-9d3bc1dd1b11)
and pass its url as an environment variable when running the server.

```sql
# Create a new PostgreSQL user and database.
sudo -u postgresql psql
create database YOUR_DB;

# It's easier if you choose the username you are logged in as.
create user YOUR_USER with encrypted password 'YOUR_PASSWORD';
grant all privileges on database YOUR_DB to YOUR_USER;
```

```bash
# Use the nightly version of the Rust compiler.
rustup override set nightly

# Start the server.
# Your database url probably looks like: postgres://YOUR_USER:YOUR_PASSWORD@localhost/YOUR_DB
ROCKET_DATABASES={db={url="YOUR_DB_URL"}} cargo run
```

> **Note**: You can run the server with the `--release` flag to activate some
optimizations. You **should NOT** use this flag when working on the project,
since features like static-file caching make development harder.

### Make it more convenient

You can make running the server easier by including the database url in a file
called `Rocket.toml`, like this:

```lang-none
[global.databases]
db = { url = "YOUR_DB_URL" }
```

This allows you to run the server with just `cargo run`.

## Deploying

[This guide](docs/deployment_guide.md) can help you deploy the web app on a free
Heroku dyno, similar to the [demo page][demo-page]. Keep in mind that **the
guide does not cover adding a SSL certificate**, which is necessary if you want
to ensure that your conversations remain private.

## Contributing

You are welcome to contribute to this project on GitHub, through pull requests.

Before you do that, please read the [contribution guide](CONTRIBUTING.md).

## Open source projects used. Thanks

Personal forum was built using many other open source projects. If you want to
contribute, getting familiar with them might make your job easier. Some of these
are:

- [Rocket](https://rocket.rs)
- [mark.js](https://markjs.io)
- [Tera](https://tera.netlify.app)
- [Ammonia](https://github.com/rust-ammonia/ammonia)
- [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)
- [Serde](https://serde.rs)
- [findAndReplaceDOMText](https://github.com/padolsey/findAndReplaceDOMText)

[demo-page]: https://calm-springs-86222.herokuapp.com "The demo page"
