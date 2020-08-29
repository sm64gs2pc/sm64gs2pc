#!/bin/sh

set -eux

wasm-pack build --target web --out-dir static/pkg
cp -f ../base-patches/* static/pkg/
