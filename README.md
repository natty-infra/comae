# comae

A simple social media crossposter.

Work in progress.

## Building and running

The application expects the `DATABASE_URL` environment variable 
to point to a valid PostgreSQL database.

A Discord application token is expected in `DISCORD_TOKEN`.

For YouTube integration, a service account key is expected
in `keys/youtube-service-account.json`.

Build and run using:

```shell
$ cargo run
```