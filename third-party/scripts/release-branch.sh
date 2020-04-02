#!/bin/sh

find . -name '*.toml'|xargs sed -i "s/ci-release-2.0.0-alpha.5+6 /plasm-v$1/g"
