#!/bin/sh
################################################################################
################################################################################
##
## DESCRIPTION
##  This script implements tag making rules for Astar Frame repository. Each 
##  pallet should be tagged according to its version and used polkadot release.
##
## USAGE
## ./script/make-tags.sh [POLKADOT_VERSION] [PALLET_PATH]
##  * POLKADOT_VERSION (optional) is version to create tags, if missed
##    then current branch name will be used (suits for default branch)
##  * PALLET_PATH (optional) is path to precise pallet for making tags
##
################################################################################
################################################################################

function create_tags {
    CARGO_TOML_PATH="$2/Cargo.toml"
    if [ ! -f "$CARGO_TOML_PATH" ]; then
        echo "$CARGO_TOML_PATH does not exist"
        exit 1
    fi

    local line key value entry_regex
    entry_regex="^[[:blank:]]*([[:alnum:]_-]+)[[:blank:]]*=[[:blank:]]*('[^']+'|\"[^\"]+\"|[^#]+)"
    while read -r line; do
        [[ -n $line ]] || continue
        [[ $line =~ $entry_regex ]] || continue
        key=${BASH_REMATCH[1]}
        value=${BASH_REMATCH[2]#[\'\"]} # strip quotes
        value=${value%[\'\"]}
        value=${value%${value##*[![:blank:]]}} # strip trailing spaces

        if [[ "$key" == "name" ]]; then PKG_NAME=$value; fi
        if [[ "$key" == "version" ]]; then PKG_VERSION=$value; fi
    done < "$CARGO_TOML_PATH"

    PKG_TAG="$PKG_NAME-$PKG_VERSION/$1"
    echo -e $PKG_TAG 

    git tag $PKG_TAG
}

if [ -z "$1" ]; then
    POLKADOT_VERSION=$(git branch --show-current)
else
    POLKADOT_VERSION=$1
fi

if [ ! -z "$2" ]; then
    create_tags $POLKADOT_VERSION $2
else 
    # create tags for all pallets
    for PALLET_PATH in $(find ./frame ./precompiles -mindepth 1 -maxdepth 1 -type d 2>/dev/zero); do 
        create_tags $POLKADOT_VERSION $PALLET_PATH 
    done
fi
