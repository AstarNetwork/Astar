#!/bin/sh

find . -name '*.toml'|xargs sed -i "s/plasm-master/plasm-v$1/g"
