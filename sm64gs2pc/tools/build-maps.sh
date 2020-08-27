#!/bin/sh

for country in eu jp us
do
    for map in "$country"/**/*.map
    do
        mkdir -p maps/"$(dirname "$map")"
        cp "$map" maps/"$map"
    done
done
