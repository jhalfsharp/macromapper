#!/bin/bash
set -eu

# Starts a local web-server that servs the contents of the `web/` folder,
# which is the folder to where the web version is compiled.

cargo install basic-http-server

(cd web && basic-http-server --addr 127.0.0.1:8080 .)
# (cd docs && python3 -m http.server 8080 --bind 127.0.0.1)