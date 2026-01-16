# tewi

oomfie image board

## Development

This is written in Rust, with a simple frontend using Typescript.

### Backend

Run the backend with `just run` (or inspect the `justfile` any other time a `just` command is needed).
This assumes a database is running, which can be started with `just db`.

### Frontend

The frontend should be built with `just build`, which uses SWC to compile the Typescript files,
and processes the CSS files with LightningCSS.