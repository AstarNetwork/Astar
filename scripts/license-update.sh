#!/bin/bash
################################################################################
################################################################################
##
## DESCRIPTION
##  This script will remove the year in all license headers
##
################################################################################
################################################################################

DIRECTORIES="../bin ../chain-extensions ../pallets ../precompiles ../primitives ../rpc-tests ../runtime ../tests ../third-party ../vendor"

# Find all source files with the old year in the copyright notice in the parent directory and its subdirectories
FILES=$(find $DIRECTORIES -type f -exec grep -l -F -e "Copyright (C) 2019-2023" {} +)

# Iterate over each file and update the year
for FILE in $FILES; do
    sed -i '' -e "s/Copyright (C) 2019-2023/Copyright (C)/g" "$FILE"
    echo "Updated $FILE"
done
